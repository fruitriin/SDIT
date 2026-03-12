use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::NamedKey;
use winit::window::WindowId;

use sdit_core::grid::{Dimensions, Scroll};
use sdit_core::index::{Column, Line, Point};
use sdit_core::render::pipeline::CellPipeline;
use sdit_core::selection::{Selection, SelectionType, selected_text};
use sdit_core::terminal::TermMode;

use crate::app::{PreeditState, SditApp, SditEvent};
use crate::input::{
    is_add_session_shortcut, is_close_session_shortcut, is_copy_shortcut,
    is_detach_session_shortcut, is_new_window_shortcut, is_paste_shortcut,
    is_sidebar_toggle_shortcut, key_to_bytes, mouse_report_sgr, mouse_report_x11, pixel_to_grid,
    session_switch_direction,
};

// ---------------------------------------------------------------------------
// ApplicationHandler
// ---------------------------------------------------------------------------

impl ApplicationHandler<SditEvent> for SditApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.initialized {
            return;
        }
        self.initialized = true;
        self.create_window(event_loop);
    }

    #[allow(clippy::too_many_lines)]
    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
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
            }

            WindowEvent::KeyboardInput { event: key_event, .. } => {
                if key_event.state == ElementState::Pressed {
                    // Cmd+Shift+N (macOS) でアクティブセッションを新ウィンドウに切出し
                    if is_detach_session_shortcut(&key_event.logical_key, self.modifiers) {
                        self.detach_session_to_new_window(id, event_loop);
                        return;
                    }

                    // Cmd+N (macOS) / Ctrl+Shift+N で新規ウィンドウ
                    if is_new_window_shortcut(&key_event.logical_key, self.modifiers) {
                        self.create_window(event_loop);
                        return;
                    }

                    // Cmd+\ (macOS) / Ctrl+\ でサイドバートグル
                    if is_sidebar_toggle_shortcut(&key_event.logical_key, self.modifiers) {
                        if let Some(ws) = self.windows.get_mut(&id) {
                            ws.sidebar.toggle();
                            let sid = ws.active_session_id();
                            self.redraw_session(sid);
                        }
                        return;
                    }

                    // Cmd+T (macOS) / Ctrl+Shift+T で同一ウィンドウに新規セッション追加
                    if is_add_session_shortcut(&key_event.logical_key, self.modifiers) {
                        self.add_session_to_window(id);
                        return;
                    }

                    // Cmd+W (macOS) / Ctrl+Shift+W でアクティブセッションを閉じる
                    if is_close_session_shortcut(&key_event.logical_key, self.modifiers) {
                        let window_closed = self.remove_active_session(id);
                        if window_closed && self.windows.is_empty() {
                            event_loop.exit();
                        }
                        return;
                    }

                    // Ctrl+Tab / Cmd+Shift+] で次のセッション
                    // Ctrl+Shift+Tab / Cmd+Shift+[ で前のセッション
                    if let Some(dir) =
                        session_switch_direction(&key_event.logical_key, self.modifiers)
                    {
                        self.switch_session(id, dir);
                        return;
                    }

                    let Some(ws) = self.windows.get(&id) else { return };
                    let Some(session) = self.session_mgr.get(ws.active_session_id()) else {
                        return;
                    };

                    let mode = session
                        .term_state
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .terminal
                        .mode();

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

                    // Cmd+C: テキストコピー
                    if is_copy_shortcut(&key_event.logical_key, self.modifiers) {
                        if let Some(sel) = &self.selection {
                            let state = session
                                .term_state
                                .lock()
                                .unwrap_or_else(std::sync::PoisonError::into_inner);
                            let text = selected_text(state.terminal.grid(), sel);
                            drop(state);
                            if !text.is_empty() {
                                if let Some(cb) = &mut self.clipboard {
                                    if let Err(e) = cb.set_text(text) {
                                        log::warn!("Clipboard set_text failed: {e}");
                                    }
                                }
                            }
                        }
                        self.selection = None;
                        let sid = ws.active_session_id();
                        self.redraw_session(sid);
                        return;
                    }

                    // Cmd+V: クリップボードからペースト
                    if is_paste_shortcut(&key_event.logical_key, self.modifiers) {
                        let text = self
                            .clipboard
                            .as_mut()
                            .and_then(|cb| cb.get_text().ok())
                            .unwrap_or_default();
                        if !text.is_empty() {
                            let bracketed = mode.contains(TermMode::BRACKETED_PASTE);
                            let bytes: Vec<u8> = if bracketed {
                                // ペースト内容からブラケットシーケンスを除去
                                // (Terminal Injection via Clipboard 攻撃防止)
                                let sanitized =
                                    text.replace("\x1b[200~", "").replace("\x1b[201~", "");
                                let mut v = b"\x1b[200~".to_vec();
                                v.extend_from_slice(sanitized.as_bytes());
                                v.extend_from_slice(b"\x1b[201~");
                                v
                            } else {
                                text.into_bytes()
                            };
                            if let Err(e) = session.pty_io.write_tx.try_send(bytes) {
                                log::warn!("PTY paste write failed: {e}");
                            }
                        }
                        return;
                    }

                    // キー入力時に選択を解除
                    self.selection = None;

                    if let Some(bytes) = key_to_bytes(&key_event.logical_key, self.modifiers, mode)
                    {
                        if let Err(e) = session.pty_io.write_tx.try_send(bytes) {
                            log::warn!("PTY write channel full or closed: {e}");
                        }
                    }
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_position = Some((position.x, position.y));

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

                // マウスドラッグ報告（DRAG/MOTION モード）
                if self.is_selecting {
                    if let Some(ws) = self.windows.get(&id) {
                        let metrics = *self.font_ctx.metrics();
                        let sidebar_w = ws.sidebar.width_px(metrics.cell_width);
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
                if let Some((x, y)) = self.cursor_position {
                    if let Some(ws) = self.windows.get(&id) {
                        let metrics = *self.font_ctx.metrics();
                        let sidebar_w = ws.sidebar.width_px(metrics.cell_width);
                        // サイドバー領域内のクリック
                        if ws.sidebar.visible && (x as f32) < sidebar_w {
                            if let Some(row) = ws.sidebar.hit_test(
                                y as f32,
                                metrics.cell_height,
                                ws.sessions.len(),
                            ) {
                                // ドラッグ開始を記録
                                self.drag_source_row = Some(row);
                                // クリックでセッション切替
                                if row != ws.active_index {
                                    let ws = self.windows.get_mut(&id).unwrap();
                                    ws.active_index = row;
                                    let sid = ws.active_session_id();
                                    self.redraw_session(sid);
                                }
                            }
                        } else {
                            // ターミナル領域: マウスモード確認
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
                                let (col, row) = pixel_to_grid(
                                    x,
                                    y,
                                    metrics.cell_width,
                                    metrics.cell_height,
                                    sidebar_w,
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
                                );
                                let now = std::time::Instant::now();
                                let is_same_pos = self.last_click_pos == Some((col, row));
                                let is_fast = self
                                    .last_click_time
                                    .is_some_and(|t| t.elapsed().as_millis() < 400);
                                if is_fast && is_same_pos {
                                    self.click_count = self.click_count.saturating_add(1).min(3);
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
                                    let (start, end) = expand_word(grid, row, col);
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
            }

            WindowEvent::MouseWheel { delta, .. } => {
                // スクロール量を行数に変換
                let lines: isize = match delta {
                    MouseScrollDelta::LineDelta(_, y) => {
                        if y > 0.0 {
                            -(y.ceil() as isize)
                        } else {
                            (-y).ceil() as isize
                        }
                    }
                    MouseScrollDelta::PixelDelta(pos) => {
                        let y = pos.y;
                        if y > 0.0 {
                            // 上方向スクロール（履歴へ）: 正の delta
                            1
                        } else if y < 0.0 {
                            // 下方向スクロール（最新へ）: 負の delta
                            -1
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
                            let (col, row) = pixel_to_grid(
                                x,
                                y,
                                metrics.cell_width,
                                metrics.cell_height,
                                sidebar_w,
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

            WindowEvent::RedrawRequested => {
                let Some(ws) = self.windows.get(&id) else { return };
                let pipelines: Vec<&CellPipeline> = if ws.sidebar.visible {
                    vec![&ws.sidebar_pipeline, &ws.cell_pipeline]
                } else {
                    vec![&ws.cell_pipeline]
                };
                match ws.gpu.render_frame(&pipelines, self.colors.background) {
                    Ok(()) => {}
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        if let Some(ws) = self.windows.get_mut(&id) {
                            let (w, h) =
                                (ws.gpu.surface_config.width, ws.gpu.surface_config.height);
                            ws.gpu.resize(w, h);
                        }
                    }
                    Err(e) => log::error!("Render error: {e}"),
                }
                // SDIT_SMOKE_TEST=1: 1フレーム描画完了後に正常終了する。
                if self.smoke_test {
                    log::info!("smoke_test: 1 frame rendered, exiting");
                    event_loop.exit();
                }
            }

            WindowEvent::Ime(winit::event::Ime::Commit(text)) => {
                let Some(ws) = self.windows.get(&id) else { return };
                let sid = ws.active_session_id();
                let Some(session) = self.session_mgr.get(sid) else { return };

                let mode = session
                    .term_state
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .terminal
                    .mode();

                let bytes: Vec<u8> = if text.len() > 1 && mode.contains(TermMode::BRACKETED_PASTE) {
                    // ブラケットペーストモード: インジェクション攻撃防止のためサニタイズ
                    let sanitized = text.replace("\x1b[200~", "").replace("\x1b[201~", "");
                    let mut v = b"\x1b[200~".to_vec();
                    v.extend_from_slice(sanitized.as_bytes());
                    v.extend_from_slice(b"\x1b[201~");
                    v
                } else {
                    text.into_bytes()
                };
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

            _ => {}
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: SditEvent) {
        match event {
            SditEvent::PtyOutput(session_id) => {
                self.redraw_session(session_id);
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
    }
}

// ---------------------------------------------------------------------------
// 単語境界拡張ヘルパー
// ---------------------------------------------------------------------------

/// グリッドの `(row, col)` から単語の開始・終了列インデックスを返す。
///
/// 空白・記号を区切り文字として扱い、連続する英数字・その他の文字を「単語」とみなす。
fn expand_word(
    grid: &sdit_core::grid::Grid<sdit_core::grid::Cell>,
    row: usize,
    col: usize,
) -> (usize, usize) {
    use sdit_core::grid::Dimensions as _;
    use sdit_core::index::{Column, Line};

    let cols = grid.columns();
    if cols == 0 {
        return (col, col);
    }
    let col = col.min(cols - 1);

    // 区切り文字セット
    let is_separator =
        |c: char| c.is_ascii_whitespace() || " \t!@#$%^&*()-=+[]{}|;:'\",.<>?/\\`~".contains(c);

    // 起点セルの文字を取得
    #[allow(clippy::cast_possible_wrap)]
    let origin_cell = &grid[Point::new(Line(row as i32), Column(col))];
    let origin_is_sep = is_separator(origin_cell.c);

    // 左方向に拡張
    let mut start = col;
    loop {
        if start == 0 {
            break;
        }
        #[allow(clippy::cast_possible_wrap)]
        let c = grid[Point::new(Line(row as i32), Column(start - 1))].c;
        if is_separator(c) != origin_is_sep {
            break;
        }
        start -= 1;
    }

    // 右方向に拡張
    let mut end = col;
    loop {
        if end + 1 >= cols {
            break;
        }
        #[allow(clippy::cast_possible_wrap)]
        let c = grid[Point::new(Line(row as i32), Column(end + 1))].c;
        if is_separator(c) != origin_is_sep {
            break;
        }
        end += 1;
    }

    (start, end)
}
