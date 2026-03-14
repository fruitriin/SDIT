use sdit_core::grid::{Dimensions, Scroll};
use winit::window::WindowId;

use crate::app::SditApp;

impl SditApp {
    /// スクロールバードラッグ中: Y 座標からスクロール位置を更新する。
    pub(crate) fn handle_scrollbar_drag(&mut self, id: WindowId, mouse_y: f64) {
        if let Some(ws) = self.windows.get(&id) {
            let padding_y = f32::from(self.config.window.clamped_padding_y());
            let surface_height = ws.gpu.surface_config.height as f32;
            let viewport_height = (surface_height - 2.0 * padding_y).max(1.0);
            let y_in_viewport = (mouse_y as f32 - padding_y).clamp(0.0, viewport_height);
            let y_ratio = y_in_viewport / viewport_height; // 0=top, 1=bottom
            let sid = ws.active_session_id();
            if let Some(session) = self.session_mgr.get(sid) {
                let mut state =
                    session.term_state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                let (history_size, current_offset) = {
                    let grid = state.terminal.grid();
                    (grid.history_size(), grid.display_offset())
                };
                if history_size > 0 {
                    // y_ratio=0 → 最上部 (scroll_ratio=1.0, display_offset=history_size)
                    // y_ratio=1 → 最下部 (scroll_ratio=0.0, display_offset=0)
                    let scroll_ratio = (1.0 - y_ratio).clamp(0.0, 1.0);
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let new_offset = (scroll_ratio * history_size as f32).round() as usize;
                    let new_offset = new_offset.min(history_size);
                    let delta = new_offset as isize - current_offset as isize;
                    if delta != 0 {
                        state.terminal.grid_mut().scroll_display(Scroll::Delta(-delta));
                    }
                }
            }
            self.redraw_session(sid);
        }
    }

    /// スクロールバー領域のクリックを処理する。
    ///
    /// クリックがスクロールバー領域内であれば `scrollbar_dragging = true` を設定し、
    /// スクロール位置を更新して `true` を返す。
    /// 領域外なら `false` を返す。
    pub(crate) fn handle_scrollbar_click(
        &mut self,
        id: WindowId,
        mouse_x: f64,
        mouse_y: f64,
    ) -> bool {
        if !self.config.scrollbar.enabled {
            return false;
        }
        let Some(ws) = self.windows.get(&id) else { return false };

        let metrics = *self.font_ctx.metrics();
        let sidebar_w = ws.sidebar.width_px(metrics.cell_width);
        let padding_x = f32::from(self.config.window.clamped_padding_x());
        let padding_y = f32::from(self.config.window.clamped_padding_y());
        let surface_width = ws.gpu.surface_config.width as f32;
        let surface_height = ws.gpu.surface_config.height as f32;
        let sid = ws.active_session_id();

        // グリッド列数を取得してスクロールバー列の X 境界を計算
        let (cols, history_size): (usize, usize) = {
            let session = self.session_mgr.get(sid).expect("session exists");
            let state =
                session.term_state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let g = state.terminal.grid();
            (g.columns(), g.history_size())
        };
        // スクロールバー列は右端1列
        let scrollbar_x_start =
            sidebar_w + padding_x + (cols.saturating_sub(1)) as f32 * metrics.cell_width;
        let is_in_scrollbar = history_size > 0
            && (mouse_x as f32) >= scrollbar_x_start
            && (mouse_x as f32) < surface_width;
        let is_in_y =
            (mouse_y as f32) >= padding_y && (mouse_y as f32) < surface_height - padding_y;

        if is_in_scrollbar && is_in_y {
            self.scrollbar_dragging = true;
            // クリック位置からスクロール位置を計算
            let viewport_height = (surface_height - 2.0 * padding_y).max(1.0);
            let y_in_viewport = (mouse_y as f32 - padding_y).clamp(0.0, viewport_height);
            let y_ratio = y_in_viewport / viewport_height;
            let scroll_ratio = (1.0 - y_ratio).clamp(0.0, 1.0);
            if let Some(session) = self.session_mgr.get(sid) {
                let mut state =
                    session.term_state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                let (history, current_offset) = {
                    let grid = state.terminal.grid();
                    (grid.history_size(), grid.display_offset())
                };
                if history > 0 {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let new_offset = (scroll_ratio * history as f32).round() as usize;
                    let new_offset = new_offset.min(history);
                    let delta = new_offset as isize - current_offset as isize;
                    if delta != 0 {
                        state.terminal.grid_mut().scroll_display(Scroll::Delta(-delta));
                    }
                }
            }
            self.redraw_session(sid);
            true
        } else {
            false
        }
    }
}
