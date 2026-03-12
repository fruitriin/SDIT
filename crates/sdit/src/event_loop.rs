use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;

use sdit_core::render::pipeline::CellPipeline;

use crate::app::{SditApp, SditEvent};
use crate::input::{
    is_add_session_shortcut, is_close_session_shortcut, is_detach_session_shortcut,
    is_new_window_shortcut, is_sidebar_toggle_shortcut, key_to_bytes, session_switch_direction,
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

                    // キー入力時に選択を解除
                    self.selection_start = None;
                    self.selection_end = None;

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

                // テキスト選択中: selection_end を更新
                if self.is_selecting {
                    if let Some(ws) = self.windows.get(&id) {
                        let metrics = *self.font_ctx.metrics();
                        let sidebar_w = ws.sidebar.width_px(metrics.cell_width);
                        let term_x = position.x as f32 - sidebar_w;
                        let col = (term_x / metrics.cell_width).floor().max(0.0) as usize;
                        let row =
                            (position.y as f32 / metrics.cell_height).floor().max(0.0) as usize;
                        self.selection_end = Some((col, row));
                        let sid = ws.active_session_id();
                        self.redraw_session(sid);
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
                            // ターミナル領域: テキスト選択開始
                            let term_x = x as f32 - sidebar_w;
                            let col = (term_x / metrics.cell_width).floor().max(0.0) as usize;
                            let row = (y as f32 / metrics.cell_height).floor().max(0.0) as usize;
                            self.selection_start = Some((col, row));
                            self.selection_end = Some((col, row));
                            self.is_selecting = true;
                            let sid = ws.active_session_id();
                            self.redraw_session(sid);
                        }
                    }
                }
            }

            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
                self.drag_source_row = None;
                self.is_selecting = false;
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

            _ => {}
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: SditEvent) {
        match event {
            SditEvent::PtyOutput(session_id) => {
                self.redraw_session(session_id);
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
