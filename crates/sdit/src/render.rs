use sdit_core::grid::Dimensions;
use sdit_core::pty::PtySize;
use sdit_core::render::pipeline::CellVertex;
use sdit_core::session::SessionId;
use sdit_core::terminal::{CursorStyle, TermMode};
use winit::window::WindowId;

use crate::app::SditApp;
use crate::window::{build_sidebar_cells, calc_grid_size};

impl SditApp {
    /// PTY 出力があったときに Terminal の Grid から GPU バッファを更新する。
    ///
    /// 非アクティブセッションの出力は描画をスキップする（Terminal には蓄積される）。
    #[allow(clippy::too_many_lines)]
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

        let selection = self.selection.as_ref().map(|sel| sel.to_tuple(grid_cols));
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

        // IME カーソル位置を通知
        {
            let preedit_width: usize =
                self.preedit.as_ref().map_or(0, |p| p.text.chars().map(char_cell_width).sum());
            let ime_col = cursor_col + preedit_width;
            let ime_x = sidebar_width_px + (ime_col as f32 * metrics.cell_width);
            let ime_y = cursor_row as f32 * metrics.cell_height;
            ws.window.set_ime_cursor_area(
                winit::dpi::PhysicalPosition::new(f64::from(ime_x), f64::from(ime_y)),
                winit::dpi::PhysicalSize::new(
                    f64::from(metrics.cell_width * 2.0),
                    f64::from(metrics.cell_height),
                ),
            );
        }

        // プリエディット描画: カーソル位置から文字を上書き
        if let Some(ref preedit) = self.preedit {
            if !preedit.text.is_empty() {
                let atlas_size = ws.atlas.size() as f32;
                let mut col_offset = cursor_col;
                for ch in preedit.text.chars() {
                    let cell_width_count = char_cell_width(ch);

                    // グリッド範囲外なら描画しない
                    if col_offset >= grid_cols {
                        break;
                    }

                    // プリエディット背景色（通常背景より少し明るく）
                    let bg = self.colors.background;
                    let preedit_bg = [
                        (bg[0] + 0.15).min(1.0),
                        (bg[1] + 0.15).min(1.0),
                        (bg[2] + 0.15).min(1.0),
                        1.0,
                    ];
                    let fg = self.colors.foreground;

                    // グリフをラスタライズしてアトラスに配置
                    let (uv, glyph_offset, glyph_size) =
                        if let Some(entry) = self.font_ctx.rasterize_glyph(ch, &mut ws.atlas) {
                            let r = entry.region;
                            let uv = [
                                r.x as f32 / atlas_size,
                                r.y as f32 / atlas_size,
                                (r.x + r.width) as f32 / atlas_size,
                                (r.y + r.height) as f32 / atlas_size,
                            ];
                            let offset = [entry.placement_left as f32, entry.placement_top as f32];
                            let size = [r.width as f32, r.height as f32];
                            (uv, offset, size)
                        } else {
                            ([0.0_f32; 4], [0.0_f32; 2], [0.0_f32; 2])
                        };

                    let vertex = CellVertex {
                        bg: preedit_bg,
                        fg,
                        grid_pos: [col_offset as f32, cursor_row as f32],
                        uv,
                        glyph_offset,
                        glyph_size,
                        cell_width_scale: if cell_width_count == 2 { 2.0 } else { 1.0 },
                    };

                    // グリッド上のセルを上書き
                    let cell_index = cursor_row * grid_cols + col_offset;
                    ws.cell_pipeline.overwrite_cell(&ws.gpu.queue, cell_index, &vertex);

                    col_offset += cell_width_count;
                }
                // アトラス更新を GPU に送信
                ws.atlas.upload_if_dirty(&ws.gpu.queue);
            }
        }

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

// ---------------------------------------------------------------------------
// ヘルパー関数
// ---------------------------------------------------------------------------

/// Unicode 文字のセル幅を返す（全角=2、それ以外=1）。
///
/// 簡易実装: CJK 統合漢字・ひらがな・カタカナ等の範囲を全角とみなす。
/// 完全な実装には `unicode-width` クレートが必要だが、依存を追加せずに
/// 主要な CJK 範囲をカバーする。
pub(crate) fn char_cell_width(c: char) -> usize {
    let cp = c as u32;
    // 主要な全角文字の範囲
    matches!(cp,
        0x1100..=0x115F  // ハングル字母
        | 0x2E80..=0x303E  // CJK ラジカルサプリメント、CJK 記号と句読点
        | 0x3041..=0x33FF  // ひらがな、カタカナ、CJK
        | 0x3400..=0x4DBF  // CJK 統合漢字拡張A
        | 0x4E00..=0x9FFF  // CJK 統合漢字
        | 0xA000..=0xA4CF  // 彝文字
        | 0xAC00..=0xD7AF  // ハングル音節
        | 0xF900..=0xFAFF  // CJK 互換漢字
        | 0xFE10..=0xFE1F  // 縦書き形
        | 0xFE30..=0xFE4F  // CJK 互換形
        | 0xFF01..=0xFF60  // 全角英数字
        | 0xFFE0..=0xFFE6  // 全角記号
        | 0x1B000..=0x1B0FF  // 変体仮名
        | 0x1F004..=0x1F0CF  // 麻雀牌
        | 0x1F300..=0x1F9FF  // 絵文字
        | 0x20000..=0x2FFFD  // CJK 統合漢字拡張B-F
        | 0x30000..=0x3FFFD  // CJK 統合漢字拡張G-
    )
    .then_some(2)
    .unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::char_cell_width;

    // -----------------------------------------------------------------------
    // char_cell_width のテスト
    // -----------------------------------------------------------------------

    #[test]
    fn ascii_is_single_width() {
        for c in 'A'..='Z' {
            assert_eq!(char_cell_width(c), 1, "ASCII '{c}' should be width 1");
        }
        assert_eq!(char_cell_width('a'), 1);
        assert_eq!(char_cell_width('0'), 1);
        assert_eq!(char_cell_width(' '), 1);
    }

    #[test]
    fn hiragana_is_double_width() {
        // ひらがな範囲 0x3041-0x3096
        assert_eq!(char_cell_width('あ'), 2);
        assert_eq!(char_cell_width('い'), 2);
        assert_eq!(char_cell_width('う'), 2);
        assert_eq!(char_cell_width('ん'), 2);
    }

    #[test]
    fn katakana_is_double_width() {
        // カタカナ範囲 0x30A0-0x30FF
        assert_eq!(char_cell_width('ア'), 2);
        assert_eq!(char_cell_width('イ'), 2);
        assert_eq!(char_cell_width('ウ'), 2);
    }

    #[test]
    fn cjk_ideographs_are_double_width() {
        // CJK 統合漢字 0x4E00-0x9FFF
        assert_eq!(char_cell_width('日'), 2);
        assert_eq!(char_cell_width('本'), 2);
        assert_eq!(char_cell_width('語'), 2);
        assert_eq!(char_cell_width('字'), 2);
    }

    #[test]
    fn fullwidth_ascii_is_double_width() {
        // 全角英数字 0xFF01-0xFF60
        assert_eq!(char_cell_width('Ａ'), 2); // U+FF21
        assert_eq!(char_cell_width('！'), 2); // U+FF01
    }

    #[test]
    fn hangul_is_double_width() {
        // ハングル音節 0xAC00-0xD7AF
        assert_eq!(char_cell_width('가'), 2); // U+AC00
        assert_eq!(char_cell_width('힣'), 2); // U+D7A3
    }

    #[test]
    fn latin_extended_is_single_width() {
        // ラテン拡張文字は半角
        assert_eq!(char_cell_width('é'), 1); // U+00E9
        assert_eq!(char_cell_width('ñ'), 1); // U+00F1
        assert_eq!(char_cell_width('ü'), 1); // U+00FC
    }

    #[test]
    fn mixed_string_total_width() {
        // 混在文字列の幅計算
        let text = "hello世界"; // 5 ASCII + 2 CJK
        let total: usize = text.chars().map(char_cell_width).sum();
        assert_eq!(total, 5 + 4, "幅計算: ASCII=5*1 + CJK=2*2 = 9");
    }

    #[test]
    fn mixed_preedit_width() {
        // プリエディット: ひらがな混在
        let preedit = "あいう123"; // 3全角 + 3半角 = 9セル
        let total: usize = preedit.chars().map(char_cell_width).sum();
        assert_eq!(total, 3 * 2 + 3, "あいう(3*2) + 123(3*1) = 9");
    }

    #[test]
    fn emoji_is_double_width() {
        // 絵文字 0x1F300-0x1F9FF
        assert_eq!(char_cell_width('🎉'), 2); // U+1F389
        assert_eq!(char_cell_width('😀'), 2); // U+1F600
    }
}
