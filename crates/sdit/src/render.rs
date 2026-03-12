use sdit_core::grid::Dimensions;
use sdit_core::pty::PtySize;
use sdit_core::session::SessionId;
use sdit_core::terminal::{CursorStyle, TermMode};
use winit::window::WindowId;

use crate::app::SditApp;
use crate::window::{build_sidebar_cells, calc_grid_size};

impl SditApp {
    /// PTY 出力があったときに Terminal の Grid から GPU バッファを更新する。
    ///
    /// 非アクティブセッションの出力は描画をスキップする（Terminal には蓄積される）。
    pub(crate) fn redraw_session(&mut self, session_id: SessionId) {
        let Some(&window_id) = self.session_to_window.get(&session_id) else { return };
        let Some(ws) = self.windows.get_mut(&window_id) else { return };

        // 非アクティブセッションの出力は描画しない
        if ws.active_session_id() != session_id {
            return;
        }

        let Some(session) = self.session_mgr.get(session_id) else { return };

        let metrics = *self.font_ctx.metrics();
        let cell_size = [metrics.cell_width, metrics.cell_height];
        let surface_size =
            [ws.gpu.surface_config.width as f32, ws.gpu.surface_config.height as f32];
        let atlas_size_f32 = ws.atlas.size() as f32;

        // サイドバー表示中はターミナル描画を右にオフセット
        let sidebar_width_px = ws.sidebar.width_px(metrics.cell_width);

        let state_lock =
            session.term_state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let grid = state_lock.terminal.grid();

        let grid_rows = grid.screen_lines();
        let grid_cols = grid.columns();
        let needed = grid_rows * grid_cols;
        ws.cell_pipeline.ensure_capacity(&ws.gpu.device, needed);

        // カーソル位置を取得
        let cursor_col = grid.cursor.point.column.0;
        #[allow(clippy::cast_sign_loss)]
        let cursor_row = grid.cursor.point.line.0 as usize;

        // カーソルスタイルと点滅状態を取得
        let cursor_style = state_lock.terminal.cursor_style();
        let cursor_visible = state_lock.terminal.mode().contains(TermMode::SHOW_CURSOR)
            && (!state_lock.terminal.cursor_blinking() || self.cursor_blink_visible);
        let cursor_pos = if cursor_visible {
            match cursor_style {
                CursorStyle::Block => Some((cursor_col, cursor_row)),
                _ => None, // Underline/Bar は将来 Phase で描画実装
            }
        } else {
            None
        };

        // ウィンドウタイトルを取得（state_lock drop 前）
        let title = state_lock.terminal.title().map(std::borrow::ToOwned::to_owned);

        let selection = match (self.selection_start, self.selection_end) {
            (Some(s), Some(e)) => Some((s, e)),
            _ => None,
        };
        ws.cell_pipeline.update_from_grid(
            &ws.gpu.queue,
            grid,
            &mut self.font_ctx,
            &mut ws.atlas,
            atlas_size_f32,
            cell_size,
            surface_size,
            cursor_pos,
            selection,
        );
        drop(state_lock);

        // ウィンドウタイトルを反映
        if let Some(title) = title {
            ws.window.set_title(&title);
        }

        // ターミナルパイプラインの origin_x を設定
        let rows_f32 = grid_rows as f32;
        let cols_f32 = grid_cols as f32;
        ws.cell_pipeline.update_uniforms(
            &ws.gpu.queue,
            cell_size,
            [cols_f32, rows_f32],
            surface_size,
            atlas_size_f32,
            sidebar_width_px,
        );

        // サイドバー描画
        if ws.sidebar.visible {
            let sidebar_cells = build_sidebar_cells(
                &ws.sessions,
                ws.active_index,
                &ws.sidebar,
                &metrics,
                surface_size,
                &mut self.font_ctx,
                &mut ws.atlas,
                &self.colors,
            );
            let sidebar_rows = (surface_size[1] / metrics.cell_height).floor().max(1.0) as usize;
            ws.sidebar_pipeline.ensure_capacity(&ws.gpu.device, sidebar_cells.len());
            ws.sidebar_pipeline.update_cells(&ws.gpu.queue, &sidebar_cells);
            ws.sidebar_pipeline.update_uniforms(
                &ws.gpu.queue,
                cell_size,
                [ws.sidebar.width_cells as f32, sidebar_rows as f32],
                surface_size,
                atlas_size_f32,
                0.0, // サイドバー自体は origin_x = 0
            );
        }

        ws.atlas.upload_if_dirty(&ws.gpu.queue);
        ws.window.request_redraw();
    }

    /// ウィンドウリサイズ時に GPU・Terminal を更新する。
    ///
    /// 全セッションの Terminal と PTY をリサイズする。
    pub(crate) fn handle_resize(&mut self, window_id: WindowId, width: u32, height: u32) {
        let Some(ws) = self.windows.get_mut(&window_id) else { return };
        ws.gpu.resize(width, height);

        let metrics = *self.font_ctx.metrics();
        let sidebar_w = ws.sidebar.width_px(metrics.cell_width);
        let term_width = (width as f32 - sidebar_w).max(0.0);
        let (cols, rows) =
            calc_grid_size(term_width, height as f32, metrics.cell_width, metrics.cell_height);

        let session_ids: Vec<SessionId> = ws.sessions.clone();
        for sid in session_ids {
            if let Some(session) = self.session_mgr.get(sid) {
                let mut state =
                    session.term_state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                state.terminal.resize(rows, cols);
                drop(state);

                let pty_size =
                    PtySize::new(rows.try_into().unwrap_or(24), cols.try_into().unwrap_or(80));
                session.resize_pty(pty_size);
            }
        }
    }
}
