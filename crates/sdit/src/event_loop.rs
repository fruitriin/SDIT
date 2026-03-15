use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::NamedKey;
use winit::window::WindowId;

use sdit_core::grid::{Dimensions, Scroll};
use sdit_core::index::{Column, Line, Point};
use sdit_core::render::pipeline::CellPipeline;
use sdit_core::selection::{Selection, SelectionType};
use sdit_core::terminal::TermMode;

use crate::app::{
    PendingClose, PreeditState, SditApp, SditEvent, confirm_unsafe_paste, ime_commit_to_bytes,
    is_unsafe_paste, wrap_bracketed_paste,
};
use sdit_core::config::CommandNotifyMode;
use sdit_core::config::keybinds::Action;
use sdit_core::session::{AppSnapshot, WindowGeometry};

use crate::cwd_utils::{parse_osc7_cwd, trim_trailing_whitespace};
use crate::input::{
    is_url_modifier, key_to_bytes, mouse_report_sgr, mouse_report_x11, pixel_to_grid,
    resolve_action,
};
use crate::selection_utils::expand_word;

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
                        if let Some((action, _unconsumed)) = resolve_action(
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

                    // --- Action-based ショートカット dispatch ---
                    // unconsumed = true のアクションは実行後も PTY にキーを転送する
                    if let Some((action, unconsumed)) = resolve_action(
                        &key_event.logical_key,
                        self.modifiers,
                        &self.config.keybinds,
                    ) {
                        self.handle_action(action, id, event_loop);
                        if !unconsumed {
                            return;
                        }
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
                self.cursor_position = Some((position.x, position.y));

                // hide_when_typing: マウス移動時にカーソルを再表示する
                if self.cursor_hidden {
                    if let Some(ws) = self.windows.get(&id) {
                        ws.window.set_cursor_visible(true);
                    }
                    self.cursor_hidden = false;
                }

                // URL ホバー更新（Cmd/Ctrl が押されている場合）
                if is_url_modifier(self.modifiers) {
                    self.update_url_hover(id);
                }

                // ドラッグ中: サイドバー内で行が変わったらタブ順序を入れ替え
                if let Some(drag_row) = self.drag_source_row {
                    if let Some(ws) = self.windows.get_mut(&id) {
                        let metrics = *self.font_ctx.metrics();
                        if let Some(target_row) = ws.sidebar.hit_test(
                            position.y as f32,
                            metrics.cell_height,
                            ws.sessions.len(),
                        ) {
                            if target_row != drag_row {
                                ws.sessions.swap(drag_row, target_row);
                                // active_index を追従させる
                                if ws.active_index == drag_row {
                                    ws.active_index = target_row;
                                } else if ws.active_index == target_row {
                                    ws.active_index = drag_row;
                                }
                                self.drag_source_row = Some(target_row);
                                let sid = ws.active_session_id();
                                self.redraw_session(sid);
                            }
                        }
                    }
                }

                // スクロールバードラッグ中: Y 座標からスクロール位置を更新
                if self.scrollbar_dragging {
                    self.handle_scrollbar_drag(id, position.y);
                }

                // マウスドラッグ報告（DRAG/MOTION モード）
                if self.is_selecting {
                    if let Some(ws) = self.windows.get(&id) {
                        let metrics = *self.font_ctx.metrics();
                        let sidebar_w = ws.sidebar.width_px(metrics.cell_width);
                        let padding_x = f32::from(self.config.window.clamped_padding_x());
                        let padding_y = f32::from(self.config.window.clamped_padding_y());
                        let sid = ws.active_session_id();
                        let mouse_drag;
                        let use_sgr;
                        {
                            let session = self.session_mgr.get(sid).expect("session exists");
                            let state = session
                                .term_state
                                .lock()
                                .unwrap_or_else(std::sync::PoisonError::into_inner);
                            let mode = state.terminal.mode();
                            mouse_drag = mode.intersects(
                                TermMode::MOUSE_REPORT_DRAG | TermMode::MOUSE_REPORT_MOTION,
                            );
                            use_sgr = mode.contains(TermMode::SGR_MOUSE);
                        }
                        if mouse_drag {
                            let (col, row) = pixel_to_grid(
                                position.x,
                                position.y,
                                metrics.cell_width,
                                metrics.cell_height,
                                sidebar_w,
                                padding_x,
                                padding_y,
                            );
                            // button=32 = 左ボタン押しながらドラッグ
                            let bytes = if use_sgr {
                                mouse_report_sgr(32, col, row, true)
                            } else {
                                mouse_report_x11(32, col, row)
                            };
                            if let Some(session) = self.session_mgr.get(sid) {
                                if let Err(e) = session.pty_io.write_tx.try_send(bytes) {
                                    log::warn!("Mouse drag PTY write failed: {e}");
                                }
                            }
                        } else {
                            // テキスト選択中: selection.end を更新
                            let (col, row) = pixel_to_grid(
                                position.x,
                                position.y,
                                metrics.cell_width,
                                metrics.cell_height,
                                sidebar_w,
                                padding_x,
                                padding_y,
                            );
                            if let Some(sel) = &mut self.selection {
                                // row は screen_lines の範囲内なので i32 に収まる。
                                #[allow(clippy::cast_possible_wrap)]
                                let new_end = Point::new(Line(row as i32), Column(col));
                                sel.end = new_end;
                            }
                            self.redraw_session(sid);
                        }
                    }
                }
            }

            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if self.just_focused.remove(&id) {
                    return;
                }
                if let Some((x, y)) = self.cursor_position {
                    // スクロールバー・サイドバー判定に必要な情報を先に取り出す（借用解放のため）
                    let click_area_info = self.windows.get(&id).map(|ws| {
                        let metrics = *self.font_ctx.metrics();
                        let sidebar_w = ws.sidebar.width_px(metrics.cell_width);
                        let padding_x = f32::from(self.config.window.clamped_padding_x());
                        let padding_y = f32::from(self.config.window.clamped_padding_y());
                        let surface_width = ws.gpu.surface_config.width as f32;
                        let surface_height = ws.gpu.surface_config.height as f32;
                        let is_sidebar = ws.sidebar.visible && (x as f32) < sidebar_w;
                        let sidebar_row = if is_sidebar {
                            ws.sidebar.hit_test(y as f32, metrics.cell_height, ws.sessions.len())
                        } else {
                            None
                        };
                        (
                            is_sidebar,
                            sidebar_row,
                            metrics,
                            sidebar_w,
                            padding_x,
                            padding_y,
                            surface_width,
                            surface_height,
                        )
                    });
                    if let Some((
                        is_sidebar,
                        sidebar_row,
                        metrics,
                        sidebar_w,
                        padding_x,
                        padding_y,
                        surface_width,
                        surface_height,
                    )) = click_area_info
                    {
                        if is_sidebar {
                            if let Some(row) = sidebar_row {
                                self.handle_sidebar_click(id, row);
                                // handle_sidebar_click 内で ws を借用するため、早期リターン
                            }
                        } else {
                            // スクロールバー領域クリック処理（ws の借用なし）
                            if self.config.scrollbar.enabled {
                                let sid = self.windows.get(&id).map(|ws| ws.active_session_id());
                                if let Some(sid) = sid {
                                    let (cols, history_size): (usize, usize) = {
                                        let session =
                                            self.session_mgr.get(sid).expect("session exists");
                                        let state = session
                                            .term_state
                                            .lock()
                                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                                        let g = state.terminal.grid();
                                        use sdit_core::grid::Dimensions;
                                        (g.columns(), g.history_size())
                                    };
                                    let scrollbar_x_start = sidebar_w
                                        + padding_x
                                        + (cols.saturating_sub(1)) as f32 * metrics.cell_width;
                                    let is_in_scrollbar = history_size > 0
                                        && (x as f32) >= scrollbar_x_start
                                        && (x as f32) < surface_width;
                                    let is_in_y = (y as f32) >= padding_y
                                        && (y as f32) < surface_height - padding_y;
                                    if is_in_scrollbar && is_in_y {
                                        self.handle_scrollbar_click(id, x, y);
                                        return;
                                    }
                                }
                            }

                            // 以下は ws の借用が必要なため再借用する
                            if let Some(ws) = self.windows.get(&id) {
                                // URL Cmd+Click 処理
                                if is_url_modifier(self.modifiers) {
                                    let metrics = *self.font_ctx.metrics();
                                    let sidebar_w = ws.sidebar.width_px(metrics.cell_width);
                                    let padding_x =
                                        f32::from(self.config.window.clamped_padding_x());
                                    let padding_y =
                                        f32::from(self.config.window.clamped_padding_y());
                                    let (col, row) = pixel_to_grid(
                                        x,
                                        y,
                                        metrics.cell_width,
                                        metrics.cell_height,
                                        sidebar_w,
                                        padding_x,
                                        padding_y,
                                    );
                                    let sid = ws.active_session_id();
                                    let url = {
                                        let session =
                                            self.session_mgr.get(sid).expect("session exists");
                                        let state = session
                                            .term_state
                                            .lock()
                                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                                        let grid = state.terminal.grid();
                                        let rows = grid.screen_lines();
                                        let cols = grid.columns();
                                        if row < rows && col < cols {
                                            let line_cells: Vec<_> = (0..cols)
                                                .map(|c| {
                                                    #[allow(clippy::cast_possible_wrap)]
                                                    grid[Point::new(Line(row as i32), Column(c))]
                                                        .clone()
                                                })
                                                .collect();
                                            self.url_detector.find_url_at(&line_cells, col)
                                        } else {
                                            None
                                        }
                                    };
                                    if let Some(url) = url {
                                        // セキュリティ: 危険なスキームを拒否 + 制御文字なしを確認
                                        // カスタムリンク（vscode:// 等）も許可するが
                                        // javascript: / data: / file: / vbscript: は拒否
                                        // （extract_url_from_action 内でも拒否済み。多層防御）
                                        let lower_url = url.to_ascii_lowercase();
                                        let is_safe = !lower_url.starts_with("javascript:")
                                            && !lower_url.starts_with("data:")
                                            && !lower_url.starts_with("file:")
                                            && !lower_url.starts_with("vbscript:")
                                            && url.bytes().all(|b| b >= 0x20 && b != 0x7F);
                                        if is_safe {
                                            #[cfg(target_os = "macos")]
                                            {
                                                if let Err(e) = std::process::Command::new("open")
                                                    .arg(&url)
                                                    .spawn()
                                                {
                                                    log::warn!("URL open failed: {e}");
                                                }
                                            }
                                            #[cfg(not(target_os = "macos"))]
                                            {
                                                if let Err(e) =
                                                    std::process::Command::new("xdg-open")
                                                        .arg(&url)
                                                        .spawn()
                                                {
                                                    log::warn!("URL open failed: {e}");
                                                }
                                            }
                                        }
                                        return;
                                    }
                                }

                                // ターミナル領域: マウスモード確認
                                let sid = ws.active_session_id();
                                let mouse_active;
                                let use_sgr;
                                {
                                    let session =
                                        self.session_mgr.get(sid).expect("session exists");
                                    let state = session
                                        .term_state
                                        .lock()
                                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                                    mouse_active = state.terminal.mouse_mode_active();
                                    use_sgr = state.terminal.mode().contains(TermMode::SGR_MOUSE);
                                }
                                if mouse_active {
                                    let (col, row) = pixel_to_grid(
                                        x,
                                        y,
                                        metrics.cell_width,
                                        metrics.cell_height,
                                        sidebar_w,
                                        padding_x,
                                        padding_y,
                                    );
                                    let bytes = if use_sgr {
                                        mouse_report_sgr(0, col, row, true)
                                    } else {
                                        mouse_report_x11(0, col, row)
                                    };
                                    if let Some(session) = self.session_mgr.get(sid) {
                                        if let Err(e) = session.pty_io.write_tx.try_send(bytes) {
                                            log::warn!("Mouse PTY write failed: {e}");
                                        }
                                    }
                                } else {
                                    // テキスト選択開始（シングル/ダブル/トリプルクリック判定）
                                    let (col, row) = pixel_to_grid(
                                        x,
                                        y,
                                        metrics.cell_width,
                                        metrics.cell_height,
                                        sidebar_w,
                                        padding_x,
                                        padding_y,
                                    );
                                    let now = std::time::Instant::now();
                                    let is_same_pos = self.last_click_pos == Some((col, row));
                                    let interval_ms = u128::from(
                                        self.config.mouse.clamped_click_repeat_interval(),
                                    );
                                    let is_fast = self
                                        .last_click_time
                                        .is_some_and(|t| t.elapsed().as_millis() < interval_ms);
                                    if is_fast && is_same_pos {
                                        self.click_count =
                                            self.click_count.saturating_add(1).min(3);
                                    } else {
                                        self.click_count = 1;
                                    }
                                    self.last_click_time = Some(now);
                                    self.last_click_pos = Some((col, row));

                                    #[allow(clippy::cast_possible_wrap)]
                                    let point = Point::new(Line(row as i32), Column(col));
                                    let sel_type = match self.click_count {
                                        3 => SelectionType::Lines,
                                        2 => SelectionType::Word,
                                        _ => SelectionType::Simple,
                                    };
                                    let mut sel = Selection::new(sel_type, point);

                                    // ダブルクリック: 単語境界まで選択を拡張
                                    if sel_type == SelectionType::Word {
                                        let session =
                                            self.session_mgr.get(sid).expect("session exists");
                                        let state = session
                                            .term_state
                                            .lock()
                                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                                        let grid = state.terminal.grid();
                                        let word_chars =
                                            self.config.selection.clamped_word_chars().to_owned();
                                        let (start, end) = expand_word(grid, row, col, &word_chars);
                                        #[allow(clippy::cast_possible_wrap)]
                                        {
                                            sel.start = Point::new(Line(row as i32), Column(start));
                                            sel.end = Point::new(Line(row as i32), Column(end));
                                        }
                                    }
                                    // トリプルクリック: 行全体を選択
                                    if sel_type == SelectionType::Lines {
                                        #[allow(clippy::cast_possible_wrap)]
                                        {
                                            sel.start = Point::new(Line(row as i32), Column(0));
                                            let session =
                                                self.session_mgr.get(sid).expect("session exists");
                                            let state = session
                                                .term_state
                                                .lock()
                                                .unwrap_or_else(std::sync::PoisonError::into_inner);
                                            let max_col =
                                                state.terminal.grid().columns().saturating_sub(1);
                                            sel.end = Point::new(Line(row as i32), Column(max_col));
                                        }
                                    }
                                    self.selection = Some(sel);
                                    self.is_selecting = sel_type == SelectionType::Simple;
                                    self.redraw_session(sid);
                                }
                            }
                        } // if let Some(ws) 再借用の閉じ
                    } // else ブロックの閉じ
                } // if let Some(click_area_info) の閉じ
            }

            #[cfg(target_os = "macos")]
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Right,
                ..
            } => {
                if let Some((x, y)) = self.cursor_position {
                    if let Some(ws) = self.windows.get(&id) {
                        let metrics = *self.font_ctx.metrics();
                        let sidebar_w = ws.sidebar.width_px(metrics.cell_width);

                        if ws.sidebar.visible && (x as f32) < sidebar_w {
                            // サイドバー領域: セッションコンテキストメニュー（設定に関わらず固定）
                            if let Some(row) = ws.sidebar.hit_test(
                                y as f32,
                                metrics.cell_height,
                                ws.sessions.len(),
                            ) {
                                // 右クリックしたセッションをアクティブにしてからメニュー表示
                                if row < ws.sessions.len() {
                                    self.windows.get_mut(&id).unwrap().active_index = row;
                                }
                                let window = self.windows[&id].window.clone();
                                crate::menu::show_context_menu_for_window(
                                    &window,
                                    &self.sidebar_ctx_menu,
                                );
                            }
                        } else {
                            // ターミナル領域: right_click_action 設定に応じて動作を切り替える
                            use sdit_core::config::RightClickAction;
                            match self.config.mouse.right_click_action {
                                RightClickAction::ContextMenu => {
                                    let window = self.windows[&id].window.clone();
                                    crate::menu::show_context_menu_for_window(
                                        &window,
                                        &self.terminal_ctx_menu,
                                    );
                                }
                                RightClickAction::Paste => {
                                    let sid = ws.active_session_id();
                                    if let Some(session) = self.session_mgr.get(sid) {
                                        let mode = session
                                            .term_state
                                            .lock()
                                            .unwrap_or_else(std::sync::PoisonError::into_inner)
                                            .terminal
                                            .mode();
                                        let text = self
                                            .clipboard
                                            .as_mut()
                                            .and_then(|cb| cb.get_text().ok())
                                            .unwrap_or_default();
                                        if !text.is_empty() {
                                            let bracketed =
                                                mode.contains(TermMode::BRACKETED_PASTE);
                                            if self.config.paste.confirm_multiline
                                                && is_unsafe_paste(&text, bracketed)
                                                && !confirm_unsafe_paste(&text)
                                            {
                                                // ユーザーがキャンセル
                                            } else {
                                                let bytes: Vec<u8> = if bracketed {
                                                    wrap_bracketed_paste(&text)
                                                } else {
                                                    text.into_bytes()
                                                };
                                                if let Err(e) =
                                                    session.pty_io.write_tx.try_send(bytes)
                                                {
                                                    log::warn!("PTY right-click paste failed: {e}");
                                                }
                                            }
                                        }
                                    }
                                }
                                RightClickAction::None => {
                                    // 何もしない
                                }
                            }
                        }
                    }
                }
            }

            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
                // マウスモード ON の場合は release イベントを SGR 形式で報告
                if let Some((x, y)) = self.cursor_position {
                    if let Some(ws) = self.windows.get(&id) {
                        let metrics = *self.font_ctx.metrics();
                        let sidebar_w = ws.sidebar.width_px(metrics.cell_width);
                        let padding_x = f32::from(self.config.window.clamped_padding_x());
                        let padding_y = f32::from(self.config.window.clamped_padding_y());
                        if !ws.sidebar.visible || (x as f32) >= sidebar_w {
                            let sid = ws.active_session_id();
                            let mouse_active;
                            let use_sgr;
                            {
                                let session = self.session_mgr.get(sid).expect("session exists");
                                let state = session
                                    .term_state
                                    .lock()
                                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                                mouse_active = state.terminal.mouse_mode_active();
                                use_sgr = state.terminal.mode().contains(TermMode::SGR_MOUSE);
                            }
                            if mouse_active && use_sgr {
                                let (col, row) = pixel_to_grid(
                                    x,
                                    y,
                                    metrics.cell_width,
                                    metrics.cell_height,
                                    sidebar_w,
                                    padding_x,
                                    padding_y,
                                );
                                let bytes = mouse_report_sgr(0, col, row, false);
                                if let Some(session) = self.session_mgr.get(sid) {
                                    if let Err(e) = session.pty_io.write_tx.try_send(bytes) {
                                        log::warn!("Mouse release PTY write failed: {e}");
                                    }
                                }
                            }
                        }
                    }
                }
                self.drag_source_row = None;
                self.is_selecting = false;
                // シングルクリック（ドラッグなし）の場合、選択をクリアする
                if let Some(ref sel) = self.selection {
                    if sel.start == sel.end {
                        self.selection = None;
                    }
                }
                self.scrollbar_dragging = false;

                // selection.save_to_clipboard: 選択完了時に自動クリップボードコピー
                if self.config.selection.save_to_clipboard {
                    if let Some(sel) = &self.selection {
                        if let Some(ws) = self.windows.get(&id) {
                            let sid = ws.active_session_id();
                            if let Some(session) = self.session_mgr.get(sid) {
                                let state = session
                                    .term_state
                                    .lock()
                                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                                let raw_text =
                                    sdit_core::selection::selected_text(state.terminal.grid(), sel);
                                drop(state);
                                if !raw_text.is_empty() {
                                    let text = if self.config.selection.trim_trailing_spaces {
                                        trim_trailing_whitespace(&raw_text)
                                    } else {
                                        raw_text
                                    };
                                    let text = self.config.selection.apply_codepoint_map(&text);
                                    if let Some(cb) = &mut self.clipboard {
                                        if let Err(e) = cb.set_text(text) {
                                            log::warn!("Auto-clipboard set_text failed: {e}");
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                // スクロール量を行数に変換
                let multiplier = self.config.scrolling.clamped_multiplier() as isize;
                let lines: isize = match delta {
                    MouseScrollDelta::LineDelta(_, y) => {
                        if y > 0.0 {
                            #[allow(clippy::cast_possible_truncation)]
                            let base = -(y.ceil() as isize);
                            base.saturating_mul(multiplier)
                        } else {
                            #[allow(clippy::cast_possible_truncation)]
                            let base = (-y).ceil() as isize;
                            base.saturating_mul(multiplier)
                        }
                    }
                    MouseScrollDelta::PixelDelta(pos) => {
                        let y = pos.y;
                        if y > 0.0 {
                            // 上方向スクロール（履歴へ）: 正の delta
                            multiplier
                        } else if y < 0.0 {
                            // 下方向スクロール（最新へ）: 負の delta
                            -multiplier
                        } else {
                            0
                        }
                    }
                };
                if lines == 0 {
                    // noop
                } else if let Some(ws) = self.windows.get(&id) {
                    let sid = ws.active_session_id();
                    let mouse_active;
                    let use_sgr;
                    {
                        let session = self.session_mgr.get(sid).expect("session exists");
                        let state = session
                            .term_state
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                        mouse_active = state.terminal.mouse_mode_active();
                        use_sgr = state.terminal.mode().contains(TermMode::SGR_MOUSE);
                    }
                    if mouse_active {
                        // マウスモード ON: スクロールを PTY に報告
                        if let Some((x, y)) = self.cursor_position {
                            let metrics = *self.font_ctx.metrics();
                            let sidebar_w = ws.sidebar.width_px(metrics.cell_width);
                            let padding_x = f32::from(self.config.window.clamped_padding_x());
                            let padding_y = f32::from(self.config.window.clamped_padding_y());
                            let (col, row) = pixel_to_grid(
                                x,
                                y,
                                metrics.cell_width,
                                metrics.cell_height,
                                sidebar_w,
                                padding_x,
                                padding_y,
                            );
                            let count = lines.unsigned_abs().clamp(1, 20);
                            for _ in 0..count {
                                // lines > 0 = 上スクロール(64), lines < 0 = 下スクロール(65)
                                let button: u8 = if lines > 0 { 65 } else { 64 };
                                let bytes = if use_sgr {
                                    mouse_report_sgr(button, col, row, true)
                                } else {
                                    mouse_report_x11(button, col, row)
                                };
                                if let Some(session) = self.session_mgr.get(sid) {
                                    if let Err(e) = session.pty_io.write_tx.try_send(bytes) {
                                        log::warn!("Mouse scroll PTY write failed: {e}");
                                    }
                                }
                            }
                        }
                    } else {
                        // マウスモード OFF: ビューポートスクロール
                        if let Some(session) = self.session_mgr.get(sid) {
                            let mut state = session
                                .term_state
                                .lock()
                                .unwrap_or_else(std::sync::PoisonError::into_inner);
                            state.terminal.grid_mut().scroll_display(Scroll::Delta(lines));
                        }
                        self.redraw_session(sid);
                    }
                }
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
