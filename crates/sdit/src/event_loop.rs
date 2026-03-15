use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::NamedKey;
use winit::window::WindowId;

use sdit_core::grid::{Dimensions, Scroll};
use sdit_core::render::pipeline::CellPipeline;
use sdit_core::terminal::TermMode;

use crate::app::{PendingClose, PreeditState, SditApp, SditEvent, ime_commit_to_bytes};
use sdit_core::config::CommandNotifyMode;
use sdit_core::config::keybinds::Action;
use sdit_core::session::{AppSnapshot, WindowGeometry};

use crate::cwd_utils::parse_osc7_cwd;
use crate::input::{is_url_modifier, key_to_bytes, resolve_action};

// ---------------------------------------------------------------------------
// ApplicationHandler
// ---------------------------------------------------------------------------

impl ApplicationHandler<SditEvent> for SditApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.initialized {
            return;
        }
        self.initialized = true;

        // winit が NSApp を初期化した後でメニューバーを登録する。
        // as_ref() を使い Menu を保持し続ける（take()→drop すると ivars が dangling になりクラッシュ）。
        #[cfg(target_os = "macos")]
        if let Some(menu_bar) = self.menu_bar.as_ref() {
            menu_bar.init_for_nsapp();
        }
        self.register_user_global_hotkeys();

        let snapshot = AppSnapshot::load(&AppSnapshot::default_path());

        // restore_session が有効かつ window_sessions に保存データがある場合は復元する。
        if self.config.window.restore_session && !snapshot.window_sessions.is_empty() {
            log::info!(
                "Restoring {} window(s) from session snapshot",
                snapshot.window_sessions.len()
            );
            for win_snap in snapshot.window_sessions {
                // active_session_index のバウンダリを検証する（geometry ムーブ前に実施）
                let active_idx = win_snap.validated_active_index();
                let geometry = win_snap.geometry.validated();
                // 最初のセッション（または CWD なし）でウィンドウを作成する
                let first_cwd = win_snap
                    .sessions
                    .first()
                    .map(|s| s.clone().validated())
                    .and_then(|s| s.working_directory)
                    .map(std::path::PathBuf::from);
                self.create_window_with_cwd(event_loop, Some(&geometry), first_cwd);

                // 2番目以降のセッションを同じウィンドウに追加する
                let window_id = self.windows.keys().last().copied();
                if let Some(wid) = window_id {
                    for session_info in win_snap.sessions.iter().skip(1) {
                        let cwd = session_info
                            .clone()
                            .validated()
                            .working_directory
                            .map(std::path::PathBuf::from);
                        self.add_session_to_window_with_cwd(wid, cwd);
                    }

                    // カスタム名を設定する
                    if let Some(ws) = self.windows.get(&wid) {
                        let session_ids: Vec<_> = ws.sessions.clone();
                        for (idx, session_info) in win_snap.sessions.iter().enumerate() {
                            let validated = session_info.clone().validated();
                            if let Some(name) = validated.custom_name {
                                if let Some(&sid) = session_ids.get(idx) {
                                    if let Some(session) = self.session_mgr.get_mut(sid) {
                                        session.custom_name = Some(name);
                                    }
                                }
                            }
                        }
                        // アクティブセッションを復元する（validated_active_index で境界保証済み）
                        if let Some(ws) = self.windows.get_mut(&wid) {
                            ws.active_index = active_idx;
                        }
                    }
                }
            }
        } else {
            // 後方互換: window_sessions がない場合は旧 windows フィールドからジオメトリのみ復元
            let geometry = snapshot.windows.first().cloned().map(WindowGeometry::validated);
            self.create_window(event_loop, geometry.as_ref());
        }

        // Quick Terminal グローバルホットキーの初期化（macOS のみ）
        #[cfg(target_os = "macos")]
        if self.config.quick_terminal.enabled {
            let mut qt_state = crate::quick_terminal::QuickTerminalState::new();
            let hotkey_str = self.config.quick_terminal.hotkey.clone();
            if let Some((manager, hotkey_id)) =
                crate::quick_terminal::register_global_hotkey(&hotkey_str)
            {
                qt_state.hotkey_manager = Some(manager);
                qt_state.hotkey_id = Some(hotkey_id);

                // グローバルホットキーイベントの受信スレッドを起動
                // 注: 登録失敗時は quick_terminal_state は Some のまま保存されるが、
                // hotkey_manager が None のためホットキーによるトグルは機能しない
                let proxy = self.event_proxy.clone();
                let registered_id = hotkey_id;
                if let Err(e) = std::thread::Builder::new()
                    .name("quick-terminal-hotkey".to_string())
                    .spawn(move || {
                        let receiver = global_hotkey::GlobalHotKeyEvent::receiver();
                        loop {
                            match receiver.recv() {
                                Ok(event) => {
                                    if event.id == registered_id
                                        && event.state == global_hotkey::HotKeyState::Pressed
                                    {
                                        if proxy.send_event(SditEvent::QuickTerminalToggle).is_err()
                                        {
                                            break; // イベントループが終了した
                                        }
                                    }
                                }
                                Err(e) => {
                                    log::error!(
                                        "quick-terminal-hotkey: receiver error: {e}. Stopping."
                                    );
                                    break;
                                }
                            }
                        }
                    })
                {
                    log::error!("quick-terminal-hotkey: failed to spawn thread: {e}");
                }
            } else {
                log::warn!(
                    "quick_terminal: hotkey '{}' registration failed. \
                     Quick Terminal toggle via hotkey will not work. \
                     Check Accessibility permissions in System Settings > Privacy & Security.",
                    hotkey_str
                );
            }
            self.quick_terminal_state = Some(qt_state);
        }
    }

    #[allow(clippy::too_many_lines)]
    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                // CloseSession と同様の確認ロジック
                if let Some(ws) = self.windows.get(&id) {
                    let sid = ws.active_session_id();
                    if self.should_confirm_close(sid) {
                        self.pending_close = Some(PendingClose::Session(sid, id));
                        self.redraw_session(sid);
                        return;
                    }
                }
                self.close_window(id);
                if self.windows.is_empty() {
                    event_loop.exit();
                }
            }

            WindowEvent::Resized(size) => {
                self.handle_resize(id, size.width, size.height);
                // リサイズ後にアクティブセッションを再描画
                if let Some(ws) = self.windows.get(&id) {
                    let sid = ws.active_session_id();
                    self.redraw_session(sid);
                }
            }

            WindowEvent::ModifiersChanged(mods) => {
                self.modifiers = mods.state();
                // URL ホバー状態を更新
                if is_url_modifier(self.modifiers) {
                    self.update_url_hover(id);
                } else if self.hovered_url.is_some() {
                    self.hovered_url = None;
                    if let Some(ws) = self.windows.get(&id) {
                        let sid = ws.active_session_id();
                        self.redraw_session(sid);
                    }
                }
            }

            WindowEvent::KeyboardInput { event: key_event, .. } => {
                if key_event.state == ElementState::Pressed {
                    // --- 閉じる確認モード中のキー入力処理（最優先で評価）---
                    if self.pending_close.is_some() {
                        use winit::keyboard::Key;
                        match &key_event.logical_key {
                            // y または Enter で確認実行
                            Key::Character(s) if s.as_str().eq_ignore_ascii_case("y") => {
                                self.execute_pending_close(event_loop);
                            }
                            Key::Named(NamedKey::Enter) => {
                                self.execute_pending_close(event_loop);
                            }
                            // n または Escape でキャンセル
                            Key::Character(s) if s.as_str().eq_ignore_ascii_case("n") => {
                                self.cancel_pending_close(id);
                            }
                            Key::Named(NamedKey::Escape) => {
                                self.cancel_pending_close(id);
                            }
                            _ => {}
                        }
                        return;
                    }

                    // --- リネームモード中のキー入力処理（最優先で評価）---
                    if self.renaming_session.is_some() {
                        if self.handle_rename_key(&key_event.logical_key, id) {
                            return;
                        }
                    }

                    // --- QuickSelect モード中のキー入力処理（通常のキー処理より先に評価）---
                    if self.quick_select.is_some() {
                        if self.handle_quick_select_key(&key_event.logical_key, id) {
                            return;
                        }
                    }

                    // --- vi モード中のキー入力処理 ---
                    if self.vi_mode.is_some() && self.handle_vi_mode_key(&key_event.logical_key, id)
                    {
                        return;
                    }

                    // --- コマンドパレットモード中のキー入力処理 ---
                    if self.command_palette.is_some() {
                        use winit::keyboard::Key;
                        match &key_event.logical_key {
                            // Escape でパレットを閉じる
                            Key::Named(NamedKey::Escape) => {
                                self.command_palette = None;
                                if let Some(ws) = self.windows.get(&id) {
                                    let sid = ws.active_session_id();
                                    self.redraw_session(sid);
                                }
                            }
                            // Enter で選択中のアクションを実行
                            Key::Named(NamedKey::Enter) => {
                                let action = self
                                    .command_palette
                                    .as_ref()
                                    .and_then(|cp| cp.selected_action());
                                self.command_palette = None;
                                if let Some(action) = action {
                                    self.handle_action(action, id, event_loop);
                                } else if let Some(ws) = self.windows.get(&id) {
                                    let sid = ws.active_session_id();
                                    self.redraw_session(sid);
                                }
                            }
                            // ↓ で選択を下に移動
                            Key::Named(NamedKey::ArrowDown) => {
                                if let Some(ref mut cp) = self.command_palette {
                                    cp.move_down();
                                }
                                if let Some(ws) = self.windows.get(&id) {
                                    let sid = ws.active_session_id();
                                    self.redraw_session(sid);
                                }
                            }
                            // ↑ で選択を上に移動
                            Key::Named(NamedKey::ArrowUp) => {
                                if let Some(ref mut cp) = self.command_palette {
                                    cp.move_up();
                                }
                                if let Some(ws) = self.windows.get(&id) {
                                    let sid = ws.active_session_id();
                                    self.redraw_session(sid);
                                }
                            }
                            // Backspace で1文字削除
                            Key::Named(NamedKey::Backspace) => {
                                if let Some(ref mut cp) = self.command_palette {
                                    cp.pop_char();
                                }
                                if let Some(ws) = self.windows.get(&id) {
                                    let sid = ws.active_session_id();
                                    self.redraw_session(sid);
                                }
                            }
                            // 通常文字入力
                            Key::Character(s) => {
                                // Cmd/Ctrl 修飾がある場合はスキップ
                                if !self.modifiers.super_key() && !self.modifiers.control_key() {
                                    if let Some(ref mut cp) = self.command_palette {
                                        cp.push_str(s.as_str());
                                    }
                                    if let Some(ws) = self.windows.get(&id) {
                                        let sid = ws.active_session_id();
                                        self.redraw_session(sid);
                                    }
                                }
                            }
                            _ => {}
                        }
                        return;
                    }

                    // --- 検索モード中のキー入力処理 ---
                    if self.search.is_some() {
                        use winit::keyboard::Key;

                        // Escape で検索バーを閉じる
                        if matches!(key_event.logical_key, Key::Named(NamedKey::Escape)) {
                            self.search = None;
                            if let Some(ws) = self.windows.get(&id) {
                                let sid = ws.active_session_id();
                                self.redraw_session(sid);
                            }
                            return;
                        }

                        // Shift+Enter で前のマッチへ（ハードコード）
                        if matches!(key_event.logical_key, Key::Named(NamedKey::Enter))
                            && self.modifiers.shift_key()
                        {
                            self.search_navigate(-1, id);
                            return;
                        }

                        // Enter で次のマッチへ（ハードコード）
                        if matches!(key_event.logical_key, Key::Named(NamedKey::Enter))
                            && !self.modifiers.shift_key()
                        {
                            self.search_navigate(1, id);
                            return;
                        }

                        // Backspace で検索クエリの最後の文字を削除
                        if matches!(key_event.logical_key, Key::Named(NamedKey::Backspace)) {
                            if let Some(ref mut search) = self.search {
                                search.query.pop();
                            }
                            self.update_search(id);
                            return;
                        }

                        // Cmd+G / Cmd+Shift+G は設定駆動（SearchNext/SearchPrev）
                        if let Some((action, _chain, _unconsumed, _performable)) = resolve_action(
                            &key_event.logical_key,
                            self.modifiers,
                            &self.config.keybinds,
                        ) {
                            match action {
                                Action::SearchNext => {
                                    self.search_navigate(1, id);
                                    return;
                                }
                                Action::SearchPrev => {
                                    self.search_navigate(-1, id);
                                    return;
                                }
                                _ => {}
                            }
                        }

                        // 通常文字入力
                        if let Key::Character(ref s) = key_event.logical_key {
                            // Cmd/Ctrl 修飾がある場合はスキップ
                            if !self.modifiers.super_key() && !self.modifiers.control_key() {
                                if let Some(ref mut search) = self.search {
                                    if search.query.len() + s.len() <= 1000 {
                                        search.query.push_str(s.as_str());
                                    }
                                }
                                self.update_search(id);
                                return;
                            }
                        }

                        // 他のキーは無視（ターミナルに送らない）
                        return;
                    }

                    if self.try_dispatch_keybind(
                        &key_event.logical_key,
                        self.modifiers,
                        id,
                        event_loop,
                    ) {
                        return;
                    }

                    let Some(ws) = self.windows.get(&id) else { return };
                    let Some(session) = self.session_mgr.get(ws.active_session_id()) else {
                        return;
                    };

                    let (mode, kitty_flags) = {
                        let state = session
                            .term_state
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                        (state.terminal.mode(), state.terminal.kitty_flags.current())
                    };

                    // Shift+PageUp / Shift+PageDown でビューポートスクロール
                    if self.modifiers.shift_key() {
                        let sid = ws.active_session_id();
                        let is_page_up = matches!(
                            key_event.logical_key,
                            winit::keyboard::Key::Named(NamedKey::PageUp)
                        );
                        let is_page_down = matches!(
                            key_event.logical_key,
                            winit::keyboard::Key::Named(NamedKey::PageDown)
                        );
                        if is_page_up || is_page_down {
                            if let Some(session) = self.session_mgr.get(sid) {
                                let mut state = session
                                    .term_state
                                    .lock()
                                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                                let half = (state.terminal.grid().screen_lines() / 2).max(1);
                                #[allow(clippy::cast_possible_wrap)]
                                let delta: isize =
                                    if is_page_up { half as isize } else { -(half as isize) };
                                state.terminal.grid_mut().scroll_display(Scroll::Delta(delta));
                            }
                            self.redraw_session(sid);
                            return;
                        }
                    }

                    // キー入力時に選択を解除
                    self.selection = None;

                    // scroll_to_bottom_on_keystroke: キー入力時にスクロールをボトムにリセット
                    // モディファイアのみのキー押下（Cmd, Ctrl, Shift, Alt 単独）では発動しない
                    if self.config.scrolling.scroll_to_bottom_on_keystroke {
                        let is_modifier_only = matches!(
                            key_event.logical_key,
                            winit::keyboard::Key::Named(
                                NamedKey::Control
                                    | NamedKey::Shift
                                    | NamedKey::Alt
                                    | NamedKey::Super
                                    | NamedKey::Hyper
                                    | NamedKey::Meta
                                    | NamedKey::CapsLock
                                    | NamedKey::Fn
                                    | NamedKey::FnLock
                                    | NamedKey::NumLock
                                    | NamedKey::ScrollLock
                                    | NamedKey::Symbol
                                    | NamedKey::SymbolLock
                            )
                        );
                        if !is_modifier_only {
                            if let Some(ws) = self.windows.get(&id) {
                                let sid = ws.active_session_id();
                                if let Some(session) = self.session_mgr.get(sid) {
                                    let mut state = session
                                        .term_state
                                        .lock()
                                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                                    if state.terminal.grid().display_offset() > 0 {
                                        state.terminal.grid_mut().scroll_display(Scroll::Bottom);
                                    }
                                }
                            }
                        }
                    }

                    // hide_when_typing: タイピング中はマウスカーソルを非表示にする
                    if self.config.mouse.hide_when_typing && !self.cursor_hidden {
                        if let Some(ws) = self.windows.get(&id) {
                            ws.window.set_cursor_visible(false);
                        }
                        self.cursor_hidden = true;
                    }

                    if let Some(bytes) =
                        key_to_bytes(&key_event.logical_key, self.modifiers, mode, kitty_flags)
                    {
                        if let Err(e) = session.pty_io.write_tx.try_send(bytes) {
                            log::warn!("PTY write channel full or closed: {e}");
                        }
                    }
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.handle_cursor_moved(id, position);
            }

            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                self.handle_mouse_left_press(id);
            }

            #[cfg(target_os = "macos")]
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Right,
                ..
            } => {
                self.handle_mouse_right_press(id);
            }

            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
                self.handle_mouse_left_release(id);
            }

            WindowEvent::MouseWheel { delta, .. } => {
                self.handle_mouse_wheel(id, delta);
            }

            WindowEvent::Focused(gained) => self.handle_focused(id, gained),

            WindowEvent::RedrawRequested => {
                let Some(ws) = self.windows.get_mut(&id) else { return };

                // ビジュアルベルの intensity を取得してクリアカラーに反映
                let opacity = self.config.window.clamped_opacity();
                let bell_intensity = ws.visual_bell.intensity();
                let clear_color = if bell_intensity > 0.0 {
                    let bg = self.colors.background;
                    // 背景色と白を intensity で補間（30% まで白を混ぜる）
                    [
                        bg[0] + (1.0 - bg[0]) * bell_intensity * 0.3,
                        bg[1] + (1.0 - bg[1]) * bell_intensity * 0.3,
                        bg[2] + (1.0 - bg[2]) * bell_intensity * 0.3,
                        opacity, // alpha はベル中も opacity を維持
                    ]
                } else {
                    let bg = self.colors.background;
                    [bg[0], bg[1], bg[2], opacity]
                };

                let pipelines: Vec<&CellPipeline> = if ws.sidebar.visible {
                    vec![&ws.sidebar_pipeline, &ws.cell_pipeline]
                } else {
                    vec![&ws.cell_pipeline]
                };
                match ws.gpu.render_frame(&pipelines, clear_color, ws.bg_pipeline.as_ref()) {
                    Ok(()) => {}
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        let (w, h) = (ws.gpu.surface_config.width, ws.gpu.surface_config.height);
                        ws.gpu.resize(w, h);
                    }
                    Err(e) => log::error!("Render error: {e}"),
                }

                // ベルアニメーション継続中なら次フレームを要求
                if bell_intensity > 0.0 {
                    ws.window.request_redraw();
                }

                // SDIT_SMOKE_TEST=1: 1フレーム描画完了後に正常終了する。
                if self.smoke_test {
                    log::info!("smoke_test: 1 frame rendered, exiting");
                    event_loop.exit();
                }
            }

            WindowEvent::Ime(winit::event::Ime::Commit(text)) => {
                // 検索モード中は検索バーにテキストを追加
                if self.search.is_some() {
                    if let Some(ref mut search) = self.search {
                        if search.query.len() + text.len() <= 1000 {
                            search.query.push_str(&text);
                        }
                    }
                    self.update_search(id);
                    return;
                }

                let Some(ws) = self.windows.get(&id) else { return };
                let sid = ws.active_session_id();
                let Some(session) = self.session_mgr.get(sid) else { return };

                let mode = session
                    .term_state
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .terminal
                    .mode();

                let bytes = ime_commit_to_bytes(text, mode.contains(TermMode::BRACKETED_PASTE));
                if let Err(e) = session.pty_io.write_tx.try_send(bytes) {
                    log::warn!("IME commit PTY write failed: {e}");
                }
                // プリエディットをクリア
                self.preedit = None;
                self.redraw_session(sid);
            }

            WindowEvent::Ime(winit::event::Ime::Preedit(text, cursor)) => {
                if text.is_empty() {
                    self.preedit = None;
                } else {
                    self.preedit = Some(PreeditState { text, cursor_offset: cursor });
                }
                // プリエディット変更時に再描画
                if let Some(ws) = self.windows.get(&id) {
                    let sid = ws.active_session_id();
                    self.redraw_session(sid);
                }
            }

            WindowEvent::Ime(winit::event::Ime::Enabled | winit::event::Ime::Disabled) => {
                // IME 有効/無効状態の変更はログのみ
                log::debug!("IME state changed for window {id:?}");
            }

            WindowEvent::CursorEntered { .. } => {
                // focus_follows_mouse: マウスが乗ったウィンドウを自動フォーカス
                if self.config.mouse.focus_follows_mouse {
                    if let Some(ws) = self.windows.get(&id) {
                        if !ws.window.has_focus() {
                            ws.window.focus_window();
                        }
                    }
                }
            }

            WindowEvent::CursorLeft { .. } => self.drag_detach_on_cursor_left(id, event_loop), // drag detach
            _ => {}
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: SditEvent) {
        match event {
            SditEvent::ConfigReloaded => {
                self.apply_config_reload();
                // 全ウィンドウ再描画
                for ws in self.windows.values() {
                    ws.window.request_redraw();
                }
            }
            SditEvent::PtyOutput(session_id) => {
                // scroll_to_bottom_on_output: 出力受信時にスクロールをボトムにリセット
                if self.config.scrolling.scroll_to_bottom_on_output {
                    if let Some(session) = self.session_mgr.get(session_id) {
                        let mut state = session
                            .term_state
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                        if state.terminal.grid().display_offset() > 0 {
                            state
                                .terminal
                                .grid_mut()
                                .scroll_display(sdit_core::grid::Scroll::Bottom);
                        }
                    }
                }
                self.redraw_session(session_id);
            }
            SditEvent::CwdChanged { session_id, cwd } => {
                // OSC 7 CWD を Session に記録する（file://hostname/path → PathBuf に変換）
                if let Some(path) = parse_osc7_cwd(&cwd) {
                    if let Some(session) = self.session_mgr.get_mut(session_id) {
                        session.cwd = Some(path.clone());
                        log::debug!("Session {} cwd updated: {cwd}", session_id.0);
                    }
                    // subtitle 設定に応じてウィンドウタイトルを更新する
                    use sdit_core::config::WindowSubtitle;
                    if self.config.window.subtitle == WindowSubtitle::WorkingDirectory {
                        if let Some(&window_id) = self.session_to_window.get(&session_id) {
                            if let Some(ws) = self.windows.get(&window_id) {
                                // アクティブセッションのタイトルのみ更新
                                if ws.active_session_id() == session_id {
                                    let display_path = path
                                        .to_str()
                                        .map(|p| {
                                            // ホームディレクトリを ~ に省略
                                            if let Some(home) = dirs::home_dir() {
                                                if let Some(home_str) = home.to_str() {
                                                    if let Some(rest) = p.strip_prefix(home_str) {
                                                        return format!("~{rest}");
                                                    }
                                                }
                                            }
                                            p.to_owned()
                                        })
                                        .unwrap_or_default();
                                    let new_title = format!("SDIT \u{2014} {display_path}");
                                    ws.window.set_title(&new_title);
                                }
                            }
                        }
                    }
                }
            }
            SditEvent::ClipboardWrite(text) => {
                if let Some(cb) = &mut self.clipboard {
                    if let Err(e) = cb.set_text(text) {
                        log::warn!("OSC 52 clipboard write failed: {e}");
                    }
                }
            }
            SditEvent::ChildExit(session_id, code) => {
                log::info!("Session {} child exited with code {code}", session_id.0);
                if let Some(&window_id) = self.session_to_window.get(&session_id) {
                    let is_only_session =
                        self.windows.get(&window_id).is_none_or(|ws| ws.sessions.len() <= 1);

                    if is_only_session {
                        self.close_window(window_id);
                    } else {
                        // 複数セッションがある場合、終了したセッションだけ除去
                        // 削除順序: ws.sessions → session_to_window → session_mgr
                        if let Some(ws) = self.windows.get_mut(&window_id) {
                            if let Some(pos) = ws.sessions.iter().position(|&s| s == session_id) {
                                ws.sessions.remove(pos);
                                if ws.active_index >= ws.sessions.len() {
                                    ws.active_index = ws.sessions.len().saturating_sub(1);
                                }
                                ws.sidebar.auto_update(ws.sessions.len());
                            }
                        }
                        self.session_to_window.remove(&session_id);
                        self.session_mgr.remove(session_id);
                        if let Some(ws) = self.windows.get(&window_id) {
                            if !ws.sessions.is_empty() {
                                let new_active = ws.active_session_id();
                                self.redraw_session(new_active);
                            }
                        }
                    }
                }
                if self.windows.is_empty() {
                    event_loop.exit();
                }
            }
            SditEvent::BellRing(session_id) => {
                let Some(&window_id) = self.session_to_window.get(&session_id) else { return };

                // ビジュアルベル
                if self.config.bell.visual {
                    if let Some(ws) = self.windows.get_mut(&window_id) {
                        ws.visual_bell.ring();
                        ws.window.request_redraw();
                    }
                }

                // Dock バウンス（ウィンドウが非フォーカスの場合のみ）
                if self.config.bell.dock_bounce {
                    if let Some(ws) = self.windows.get(&window_id) {
                        if !ws.window.has_focus() {
                            ws.window.request_user_attention(Some(
                                winit::window::UserAttentionType::Informational,
                            ));
                        }
                    }
                }
            }
            SditEvent::DesktopNotification { title, body } => {
                if self.config.notification.enabled {
                    // バックグラウンドスレッドで通知送信（イベントループをブロックしない）。
                    // AtomicBool でレート制限: 前の通知スレッドが完了するまで新規スレッドを立ち上げない。
                    let in_flight = Arc::clone(&self.notification_in_flight);
                    if in_flight
                        .compare_exchange(
                            false,
                            true,
                            std::sync::atomic::Ordering::Acquire,
                            std::sync::atomic::Ordering::Relaxed,
                        )
                        .is_ok()
                    {
                        std::thread::Builder::new()
                            .name("desktop-notify".to_string())
                            .spawn(move || {
                                if let Err(e) = notify_rust::Notification::new()
                                    .summary(&title)
                                    .body(&body)
                                    .show()
                                {
                                    log::warn!("Desktop notification failed: {e}");
                                }
                                in_flight.store(false, std::sync::atomic::Ordering::Release);
                            })
                            .ok();
                    }
                }
            }
            SditEvent::CommandFinished { session_id, elapsed_secs, exit_code } => {
                let threshold =
                    u64::from(self.config.notification.clamped_command_notify_threshold());
                if elapsed_secs >= threshold {
                    let should_notify = match self.config.notification.command_notify {
                        CommandNotifyMode::Never => false,
                        CommandNotifyMode::Always => true,
                        CommandNotifyMode::Unfocused => {
                            // session_id に対応するウィンドウがフォーカスされていない場合のみ通知
                            !self.windows.values().any(|ws| {
                                ws.sessions.contains(&session_id) && ws.window.has_focus()
                            })
                        }
                    };
                    if should_notify {
                        let exit_str = match exit_code {
                            Some(0) => "\u{2713}".to_string(),
                            Some(c) => format!("\u{2717} (exit {c})"),
                            None => "完了".to_string(),
                        };
                        let title = "コマンド終了".to_string();
                        let body = format!("{exit_str}  \u{2014} {elapsed_secs}秒");
                        let _ = self
                            .event_proxy
                            .send_event(SditEvent::DesktopNotification { title, body });
                    }
                }
            }
            SditEvent::MenuAction(action) | SditEvent::GlobalHotkeyAction(action) => {
                // フォーカスがあるウィンドウ、またはウィンドウが1つだけならそれを使う。
                let focused_window_id = self
                    .windows
                    .iter()
                    .find(|(_, ws)| ws.window.has_focus())
                    .map(|(id, _)| *id)
                    .or_else(|| self.windows.keys().next().copied());
                if let Some(wid) = focused_window_id {
                    self.handle_action(action, wid, event_loop);
                }
            }
            #[cfg(target_os = "macos")]
            SditEvent::QuickTerminalToggle => {
                self.handle_quick_terminal_toggle(event_loop);
            }
            #[cfg(not(target_os = "macos"))]
            SditEvent::QuickTerminalToggle => {
                // macOS 以外では無視する
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // カーソル点滅のチェック（500ms間隔）
        const BLINK_INTERVAL: std::time::Duration = std::time::Duration::from_millis(500);
        if self.cursor_blink_last_toggle.elapsed() >= BLINK_INTERVAL {
            self.cursor_blink_visible = !self.cursor_blink_visible;
            self.cursor_blink_last_toggle = std::time::Instant::now();
            // 点滅中のウィンドウを再描画
            for ws in self.windows.values() {
                ws.window.request_redraw();
            }
        }

        // Quick Terminal アニメーション tick（macOS のみ）
        #[cfg(target_os = "macos")]
        self.tick_quick_terminal_animation();
    }
}
