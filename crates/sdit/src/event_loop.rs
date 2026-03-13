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
    PreeditState, SditApp, SditEvent, SearchState, UrlHoverState, confirm_unsafe_paste,
    ime_commit_to_bytes, is_unsafe_paste, wrap_bracketed_paste,
};
use sdit_core::config::keybinds::Action;
use sdit_core::session::AppSnapshot;

use crate::input::{
    is_url_modifier, key_to_bytes, mouse_report_sgr, mouse_report_x11, pixel_to_grid,
    resolve_action,
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
        // 前回保存したジオメトリを復元して最初のウィンドウを作成する
        let snapshot = AppSnapshot::load(&AppSnapshot::default_path());
        let geometry =
            snapshot.windows.first().cloned().map(sdit_core::session::WindowGeometry::validated);
        self.create_window(event_loop, geometry.as_ref());
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
                        if let Some(action) = resolve_action(
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
                    if let Some(action) = resolve_action(
                        &key_event.logical_key,
                        self.modifiers,
                        &self.config.keybinds,
                    ) {
                        self.handle_action(action, id, event_loop);
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
                            // URL Cmd+Click 処理
                            if is_url_modifier(self.modifiers) {
                                let metrics = *self.font_ctx.metrics();
                                let sidebar_w = ws.sidebar.width_px(metrics.cell_width);
                                let (col, row) = pixel_to_grid(
                                    x,
                                    y,
                                    metrics.cell_width,
                                    metrics.cell_height,
                                    sidebar_w,
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
                                    // セキュリティ: http(s) スキーム + 制御文字なしを確認
                                    let is_safe = (url.starts_with("http://")
                                        || url.starts_with("https://"))
                                        && url.bytes().all(|b| b >= 0x20 && b != 0x7F);
                                    if is_safe {
                                        #[cfg(target_os = "macos")]
                                        {
                                            if let Err(e) =
                                                std::process::Command::new("open").arg(&url).spawn()
                                            {
                                                log::warn!("URL open failed: {e}");
                                            }
                                        }
                                        #[cfg(not(target_os = "macos"))]
                                        {
                                            if let Err(e) = std::process::Command::new("xdg-open")
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
                            // サイドバー領域: セッションコンテキストメニュー
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
                            // ターミナル領域: 標準コンテキストメニュー
                            let window = self.windows[&id].window.clone();
                            crate::menu::show_context_menu_for_window(
                                &window,
                                &self.terminal_ctx_menu,
                            );
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
                match ws.gpu.render_frame(&pipelines, clear_color) {
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
                    // バックグラウンドスレッドで通知送信（イベントループをブロックしない）
                    std::thread::Builder::new()
                        .name("desktop-notify".to_string())
                        .spawn(move || {
                            if let Err(e) =
                                notify_rust::Notification::new().summary(&title).body(&body).show()
                            {
                                log::warn!("Desktop notification failed: {e}");
                            }
                        })
                        .ok();
                }
            }
            SditEvent::MenuAction(action) => {
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
// アクションハンドラ
// ---------------------------------------------------------------------------

impl SditApp {
    /// アクションを実行する。
    ///
    /// キーボードショートカットとメニューバーの両方から呼び出される。
    #[allow(clippy::too_many_lines)]
    pub(crate) fn handle_action(
        &mut self,
        action: Action,
        window_id: WindowId,
        event_loop: &ActiveEventLoop,
    ) {
        match action {
            Action::DetachSession => {
                self.detach_session_to_new_window(window_id, event_loop);
            }
            Action::NewWindow => {
                self.create_window(event_loop, None);
            }
            Action::SidebarToggle => {
                if let Some(ws) = self.windows.get_mut(&window_id) {
                    ws.sidebar.toggle();
                    let sid = ws.active_session_id();
                    self.redraw_session(sid);
                }
            }
            Action::AddSession => {
                self.add_session_to_window(window_id);
            }
            Action::CloseSession => {
                let window_closed = self.remove_active_session(window_id);
                if window_closed && self.windows.is_empty() {
                    event_loop.exit();
                }
            }
            Action::NextSession => {
                self.switch_session(window_id, 1);
            }
            Action::PrevSession => {
                self.switch_session(window_id, -1);
            }
            Action::ZoomIn => {
                self.change_font_size(Some(1.0));
                for ws in self.windows.values() {
                    ws.window.request_redraw();
                }
                if let Some(ws) = self.windows.get(&window_id) {
                    let sid = ws.active_session_id();
                    self.redraw_session(sid);
                }
            }
            Action::ZoomOut => {
                self.change_font_size(Some(-1.0));
                for ws in self.windows.values() {
                    ws.window.request_redraw();
                }
                if let Some(ws) = self.windows.get(&window_id) {
                    let sid = ws.active_session_id();
                    self.redraw_session(sid);
                }
            }
            Action::ZoomReset => {
                self.change_font_size(None);
                for ws in self.windows.values() {
                    ws.window.request_redraw();
                }
                if let Some(ws) = self.windows.get(&window_id) {
                    let sid = ws.active_session_id();
                    self.redraw_session(sid);
                }
            }
            Action::Search => {
                if self.search.is_some() {
                    self.search = None;
                } else {
                    self.search = Some(SearchState::new());
                }
                if let Some(ws) = self.windows.get(&window_id) {
                    let sid = ws.active_session_id();
                    self.redraw_session(sid);
                }
            }
            // 検索モード外では SearchNext/SearchPrev は無視
            Action::SearchNext | Action::SearchPrev => {}
            Action::Copy => {
                let Some(ws) = self.windows.get(&window_id) else { return };
                let Some(session) = self.session_mgr.get(ws.active_session_id()) else {
                    return;
                };
                if let Some(sel) = &self.selection {
                    let state = session
                        .term_state
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    let text = sdit_core::selection::selected_text(state.terminal.grid(), sel);
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
            }
            Action::Paste => {
                let Some(ws) = self.windows.get(&window_id) else { return };
                let Some(session) = self.session_mgr.get(ws.active_session_id()) else {
                    return;
                };
                let mode = session
                    .term_state
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .terminal
                    .mode();
                let text =
                    self.clipboard.as_mut().and_then(|cb| cb.get_text().ok()).unwrap_or_default();
                if !text.is_empty() {
                    let bracketed = mode.contains(TermMode::BRACKETED_PASTE);

                    // Unsafe paste check
                    if self.config.paste.confirm_multiline
                        && is_unsafe_paste(&text, bracketed)
                        && !confirm_unsafe_paste(&text)
                    {
                        return; // ユーザーがキャンセル
                    }

                    let bytes: Vec<u8> =
                        if bracketed { wrap_bracketed_paste(&text) } else { text.into_bytes() };
                    if let Err(e) = session.pty_io.write_tx.try_send(bytes) {
                        log::warn!("PTY paste write failed: {e}");
                    }
                }
            }
            Action::Quit => {
                // 全ウィンドウを閉じてイベントループを終了する。
                let window_ids: Vec<WindowId> = self.windows.keys().copied().collect();
                for wid in window_ids {
                    self.close_window(wid);
                }
                event_loop.exit();
            }
            Action::About => {
                // バージョン情報をログに出力（将来はダイアログ表示に置き換える）。
                log::info!("SDIT v{}", env!("CARGO_PKG_VERSION"));
            }
            Action::Preferences => {
                // 設定ファイルが存在しない場合はコメント付きテンプレートを生成する。
                // save_with_comments は create_new(true) で排他的に作成するため TOCTOU 安全。
                let path = sdit_core::config::Config::default_path();
                let config = sdit_core::config::Config::default();
                if let Err(e) = config.save_with_comments(&path) {
                    log::warn!("Failed to create default config: {e}");
                }
                if let Err(e) = open::that(&path) {
                    log::warn!("Failed to open preferences file {}: {e}", path.display());
                }
            }
            Action::SelectAll => {
                // 全テキスト選択は将来実装。現時点ではログのみ。
                log::info!("SelectAll action triggered (not yet implemented)");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 検索ヘルパー
// ---------------------------------------------------------------------------

impl SditApp {
    /// 検索クエリでグリッドを検索し、結果を更新して再描画する。
    pub(crate) fn update_search(&mut self, window_id: WindowId) {
        let Some(ws) = self.windows.get(&window_id) else { return };
        let sid = ws.active_session_id();
        let Some(session) = self.session_mgr.get(sid) else { return };

        {
            let state =
                session.term_state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let grid = state.terminal.grid();

            if let Some(ref mut search) = self.search {
                use sdit_core::terminal::search::SearchEngine;
                search.matches = SearchEngine::search(grid, &search.query);
                if search.matches.is_empty() {
                    search.current_match = 0;
                } else {
                    search.current_match =
                        search.current_match.min(search.matches.len().saturating_sub(1));
                }
            }
        }

        self.redraw_session(sid);
    }

    /// 検索マッチ間をナビゲートする（direction: 1=次, -1=前）。
    pub(crate) fn search_navigate(&mut self, direction: i32, window_id: WindowId) {
        let Some(ws) = self.windows.get(&window_id) else { return };
        let sid = ws.active_session_id();

        if let Some(ref mut search) = self.search {
            if search.matches.is_empty() {
                return;
            }
            let len = search.matches.len();
            if direction > 0 {
                search.current_match = (search.current_match + 1) % len;
            } else {
                search.current_match =
                    if search.current_match == 0 { len - 1 } else { search.current_match - 1 };
            }

            // マッチ位置にスクロール
            let raw_row = search.matches[search.current_match].raw_row;
            if let Some(session) = self.session_mgr.get(sid) {
                use sdit_core::grid::{Dimensions, Scroll};
                use sdit_core::terminal::search::SearchEngine;
                let mut state =
                    session.term_state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                let history = state.terminal.grid().history_size();
                let screen = state.terminal.grid().screen_lines();
                let target_offset =
                    SearchEngine::display_offset_for_match(raw_row, history, screen);
                let current_offset = state.terminal.grid().display_offset();
                if target_offset != current_offset {
                    // まず Bottom（表示端）にして、必要ならデルタスクロールで調整
                    state.terminal.grid_mut().scroll_display(Scroll::Bottom);
                    if target_offset > 0 {
                        #[allow(clippy::cast_possible_wrap)]
                        state
                            .terminal
                            .grid_mut()
                            .scroll_display(Scroll::Delta(target_offset as isize));
                    }
                }
            }
        }

        self.redraw_session(sid);
    }
}

// ---------------------------------------------------------------------------
// URL ホバーヘルパー
// ---------------------------------------------------------------------------

impl SditApp {
    /// 現在のカーソル位置で URL ホバー状態を更新する。
    ///
    /// URL が見つかれば `hovered_url` を更新して再描画する。
    /// 変化がなければ再描画しない。
    pub(crate) fn update_url_hover(&mut self, window_id: winit::window::WindowId) {
        let Some((x, y)) = self.cursor_position else {
            return;
        };
        let Some(ws) = self.windows.get(&window_id) else {
            return;
        };

        let metrics = *self.font_ctx.metrics();
        let sidebar_w = ws.sidebar.width_px(metrics.cell_width);

        // サイドバー領域は対象外
        if ws.sidebar.visible && (x as f32) < sidebar_w {
            if self.hovered_url.is_some() {
                self.hovered_url = None;
                if let Some(ws) = self.windows.get(&window_id) {
                    let sid = ws.active_session_id();
                    self.redraw_session(sid);
                }
            }
            return;
        }

        let (col, row) = pixel_to_grid(x, y, metrics.cell_width, metrics.cell_height, sidebar_w);

        let Some(ws) = self.windows.get(&window_id) else {
            return;
        };
        let sid = ws.active_session_id();

        let url_match = {
            let Some(session) = self.session_mgr.get(sid) else {
                return;
            };
            let state =
                session.term_state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let grid = state.terminal.grid();
            let rows = grid.screen_lines();
            let cols = grid.columns();
            if row >= rows || col >= cols {
                None
            } else {
                let line_cells: Vec<_> = (0..cols)
                    .map(|c| {
                        #[allow(clippy::cast_possible_wrap)]
                        grid[Point::new(Line(row as i32), Column(c))].clone()
                    })
                    .collect();
                let url = self.url_detector.find_url_at(&line_cells, col);
                url.map(|u| {
                    // start_col と end_col を UrlDetector で検出
                    let matches = self.url_detector.detect_urls_in_line(&line_cells);
                    matches
                        .into_iter()
                        .find(|m| col >= m.start_col && col < m.end_col)
                        .map(|m| UrlHoverState {
                            row,
                            start_col: m.start_col,
                            end_col: m.end_col,
                            url: u.clone(),
                        })
                        .unwrap_or(UrlHoverState { row, start_col: col, end_col: col + 1, url: u })
                })
            }
        };

        // ホバー状態が変わった場合のみ更新・再描画
        let changed = match (&self.hovered_url, &url_match) {
            (None, None) => false,
            (Some(old), Some(new)) => {
                old.row != new.row || old.start_col != new.start_col || old.end_col != new.end_col
            }
            _ => true,
        };

        if changed {
            // カーソルアイコンを変更
            if let Some(ws) = self.windows.get(&window_id) {
                let icon = if url_match.is_some() {
                    winit::window::CursorIcon::Pointer
                } else {
                    winit::window::CursorIcon::Default
                };
                ws.window.set_cursor(icon);
            }
            self.hovered_url = url_match;
            self.redraw_session(sid);
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
