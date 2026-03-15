//! マウスイベントハンドラ
//!
//! WindowEvent::CursorMoved / MouseInput / MouseWheel の処理を
//! event_loop.rs から分離したモジュール。

use winit::event::MouseScrollDelta;
use winit::window::WindowId;

use sdit_core::grid::{Dimensions, Scroll};
use sdit_core::index::{Column, Line, Point};
use sdit_core::selection::{Selection, SelectionType};
use sdit_core::terminal::TermMode;

use crate::app::{SditApp, confirm_unsafe_paste, is_unsafe_paste, wrap_bracketed_paste};
use crate::cwd_utils::trim_trailing_whitespace;
use crate::input::{
    is_url_modifier, mouse_report_sgr, mouse_report_x11, pixel_to_grid,
};
use crate::selection_utils::expand_word;

impl SditApp {
    pub(crate) fn handle_cursor_moved(
        &mut self,
        id: WindowId,
        position: winit::dpi::PhysicalPosition<f64>,
    ) {
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
                    #[allow(clippy::cast_possible_wrap)]
                    let new_end = Point::new(Line(row as i32), Column(col));

                    // click_origin がある場合: ドラッグで1セル以上動いたら選択を開始
                    if let Some(origin) = self.click_origin {
                        if new_end != origin {
                            self.selection =
                                Some(Selection::new(SelectionType::Simple, origin));
                            if let Some(sel) = &mut self.selection {
                                sel.end = new_end;
                            }
                            self.click_origin = None;
                            self.redraw_session(sid);
                        }
                    } else if let Some(sel) = &mut self.selection {
                        // グリッド座標が変わった場合のみ再描画
                        if sel.end != new_end {
                            sel.end = new_end;
                            self.redraw_session(sid);
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn handle_mouse_left_press(&mut self, id: WindowId) {
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
                        // テキスト選択開始（シングル/ダブル/トリプルクリック判定）
                        // マウスモード中でもクリックカウントは常に更新する。
                        // ダブル/トリプルクリックは SDIT 側で selection を作成し、
                        // シングルクリックはマウスモード時にアプリへ転送する。
                        {
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

                            if sel_type == SelectionType::Simple {
                                if mouse_active {
                                    // マウスモード中のシングルクリック: アプリに転送
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
                                }
                                // シングルクリック: 既存の選択をクリアし、
                                // ドラッグで1セル以上移動してから新しい選択を開始する
                                self.click_origin = Some(point);
                                let had_selection = self.selection.is_some();
                                self.selection = None;
                                self.is_selecting = !mouse_active; // マウスモード中はドラッグ選択しない
                                if had_selection {
                                    self.redraw_session(sid);
                                }
                            } else {
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
                                self.click_origin = None;
                                self.selection = Some(sel);
                                self.is_selecting = false;
                                self.redraw_session(sid);
                            }
                        }
                    } // if let Some(ws) 再借用の閉じ
                } // テキスト選択ブロックの閉じ
            } // if let Some(click_area_info) の閉じ
        }
    }

    pub(crate) fn handle_mouse_left_release(&mut self, id: WindowId) {
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
        self.click_origin = None;
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

    #[cfg(target_os = "macos")]
    pub(crate) fn handle_mouse_right_press(&mut self, id: WindowId) {
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

    pub(crate) fn handle_mouse_wheel(
        &mut self,
        id: WindowId,
        delta: MouseScrollDelta,
    ) {
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
}

