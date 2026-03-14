//! URL ホバー状態の更新ヘルパー。

use sdit_core::grid::Dimensions;
use sdit_core::index::{Column, Line, Point};
use winit::window::WindowId;

use crate::app::{SditApp, UrlHoverState};
use crate::input::pixel_to_grid;

impl SditApp {
    /// 現在のカーソル位置で URL ホバー状態を更新する。
    ///
    /// URL が見つかれば `hovered_url` を更新して再描画する。
    /// 変化がなければ再描画しない。
    pub(crate) fn update_url_hover(&mut self, window_id: WindowId) {
        let Some((x, y)) = self.cursor_position else {
            return;
        };
        let Some(ws) = self.windows.get(&window_id) else {
            return;
        };

        let metrics = *self.font_ctx.metrics();
        let sidebar_w = ws.sidebar.width_px(metrics.cell_width);
        let padding_x = f32::from(self.config.window.clamped_padding_x());
        let padding_y = f32::from(self.config.window.clamped_padding_y());

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

        let (col, row) = pixel_to_grid(
            x,
            y,
            metrics.cell_width,
            metrics.cell_height,
            sidebar_w,
            padding_x,
            padding_y,
        );

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
