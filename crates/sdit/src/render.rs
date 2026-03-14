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
        let url_hover = self.hovered_url.as_ref().map(|h| (h.row, h.start_col, h.end_col));

        // 検索マッチをビューポート相対座標に変換
        let (search_highlight, current_highlight) = if let Some(ref search) = self.search {
            use sdit_core::terminal::search::SearchEngine;
            let history = grid.history_size();
            let display_offset = grid.display_offset();
            let screen = grid.screen_lines();

            let visible_matches: Vec<(usize, usize, usize)> = search
                .matches
                .iter()
                .filter_map(|m| {
                    SearchEngine::raw_row_to_viewport(m.raw_row, history, display_offset, screen)
                        .map(|vr| (vr, m.start_col, m.end_col))
                })
                .collect();

            let current = if search.matches.is_empty() {
                None
            } else {
                let cm = &search.matches[search.current_match];
                SearchEngine::raw_row_to_viewport(cm.raw_row, history, display_offset, screen)
                    .map(|vr| (vr, cm.start_col, cm.end_col))
            };

            (Some(visible_matches), current)
        } else {
            (None, None)
        };

        // カーソル色: 設定された hex 文字列をパース。パース失敗時はログ警告して None 扱い。
        let cursor_color = self.config.cursor.color.as_deref().and_then(|hex| {
            let parsed = parse_hex_color(hex);
            if parsed.is_none() {
                log::warn!("cursor.color: invalid hex color '{hex}', using default");
            }
            parsed
        });

        // 選択色: 設定された hex 文字列をパース。パース失敗時はログ警告して None 扱い。
        let selection_fg = self.config.colors.selection_foreground.as_deref().and_then(|hex| {
            let parsed = sdit_core::config::color::parse_selection_color(hex);
            if parsed.is_none() {
                log::warn!("colors.selection_foreground: invalid hex color '{hex}', using default");
            }
            parsed
        });
        let selection_bg = self.config.colors.selection_background.as_deref().and_then(|hex| {
            let parsed = sdit_core::config::color::parse_selection_color(hex);
            if parsed.is_none() {
                log::warn!("colors.selection_background: invalid hex color '{hex}', using default");
            }
            parsed
        });

        let minimum_contrast = self.config.colors.clamped_minimum_contrast();
        ws.cell_pipeline.update_from_grid(
            &ws.gpu.queue,
            grid,
            &mut self.font_ctx,
            &mut ws.atlas,
            atlas_size_f32,
            cell_size,
            surface_size,
            cursor_pos,
            cursor_color,
            selection,
            url_hover,
            search_highlight.as_deref(),
            current_highlight,
            selection_fg,
            selection_bg,
            minimum_contrast,
        );
        drop(state_lock);

        // ウィンドウタイトルを反映
        if let Some(title) = title {
            ws.window.set_title(&title);
        }

        // ターミナルパイプラインの origin_x / origin_y を設定（サイドバー + パディング）
        let rows_f32 = grid_rows as f32;
        let cols_f32 = grid_cols as f32;
        let padding_x = f32::from(self.config.window.clamped_padding_x());
        let padding_y = f32::from(self.config.window.clamped_padding_y());
        ws.cell_pipeline.update_uniforms(
            &ws.gpu.queue,
            cell_size,
            [cols_f32, rows_f32],
            surface_size,
            atlas_size_f32,
            sidebar_width_px + padding_x,
            padding_y,
        );

        // サイドバー描画
        if ws.sidebar.visible {
            // セッション名リストを構築（カスタム名があればそれを使用）
            let session_names: Vec<Option<String>> = ws
                .sessions
                .iter()
                .map(|sid| self.session_mgr.get(*sid).and_then(|s| s.custom_name.clone()))
                .collect();
            // リネームモード中の行インデックスと入力テキスト
            let renaming_row = self.renaming_session.as_ref().and_then(|(rename_sid, text)| {
                ws.sessions.iter().position(|sid| sid == rename_sid).map(|row| (row, text.as_str()))
            });
            let sidebar_cells = build_sidebar_cells(
                &ws.sessions,
                &session_names,
                renaming_row,
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
                0.0, // サイドバー自体は origin_y = 0
            );
        }

        ws.atlas.upload_if_dirty(&ws.gpu.queue);

        // IME カーソル位置を通知
        {
            let preedit_width: usize =
                self.preedit.as_ref().map_or(0, |p| p.text.chars().map(char_cell_width).sum());
            let ime_col = cursor_col + preedit_width;
            let ime_x = sidebar_width_px + padding_x + (ime_col as f32 * metrics.cell_width);
            let ime_y = padding_y + cursor_row as f32 * metrics.cell_height;
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
                        is_color_glyph: 0.0,
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

        // 検索バー描画: 最下行をオーバーレイ
        if let Some(ref search) = self.search {
            let bar_row = grid_rows.saturating_sub(1);
            let search_bg = [
                (self.colors.background[0] + 0.1).min(1.0),
                (self.colors.background[1] + 0.1).min(1.0),
                (self.colors.background[2] + 0.1).min(1.0),
                1.0,
            ];
            let fg = self.colors.foreground;

            // " > " + query + " [n/m]" を構築
            let match_info = if search.matches.is_empty() {
                if search.query.is_empty() { String::new() } else { " [0/0]".to_string() }
            } else {
                format!(" [{}/{}]", search.current_match + 1, search.matches.len())
            };
            let bar_text = format!(" > {}{}", search.query, match_info);

            let atlas_size = ws.atlas.size() as f32;
            let mut col = 0usize;
            for ch in bar_text.chars() {
                if col >= grid_cols {
                    break;
                }
                let cell_width_count = char_cell_width(ch);

                let (uv, glyph_offset, glyph_size) =
                    if let Some(entry) = self.font_ctx.rasterize_glyph(ch, &mut ws.atlas) {
                        let r = entry.region;
                        let uv = [
                            r.x as f32 / atlas_size,
                            r.y as f32 / atlas_size,
                            (r.x + r.width) as f32 / atlas_size,
                            (r.y + r.height) as f32 / atlas_size,
                        ];
                        (
                            uv,
                            [entry.placement_left as f32, entry.placement_top as f32],
                            [r.width as f32, r.height as f32],
                        )
                    } else {
                        ([0.0_f32; 4], [0.0_f32; 2], [0.0_f32; 2])
                    };

                let vertex = CellVertex {
                    bg: search_bg,
                    fg,
                    grid_pos: [col as f32, bar_row as f32],
                    uv,
                    glyph_offset,
                    glyph_size,
                    cell_width_scale: if cell_width_count == 2 { 2.0 } else { 1.0 },
                    is_color_glyph: 0.0,
                };

                let cell_index = bar_row * grid_cols + col;
                ws.cell_pipeline.overwrite_cell(&ws.gpu.queue, cell_index, &vertex);
                col += cell_width_count;
            }

            // 残りのセルを検索バー背景で埋める
            while col < grid_cols {
                let vertex = CellVertex {
                    bg: search_bg,
                    fg: [0.0; 4],
                    grid_pos: [col as f32, bar_row as f32],
                    uv: [0.0; 4],
                    glyph_offset: [0.0; 2],
                    glyph_size: [0.0; 2],
                    cell_width_scale: 1.0,
                    is_color_glyph: 0.0,
                };
                let cell_index = bar_row * grid_cols + col;
                ws.cell_pipeline.overwrite_cell(&ws.gpu.queue, cell_index, &vertex);
                col += 1;
            }

            ws.atlas.upload_if_dirty(&ws.gpu.queue);
        }

        // QuickSelect オーバーレイ描画
        if let Some(ref qs) = self.quick_select {
            let atlas_size = ws.atlas.size() as f32;

            // マッチ背景色（青緑系ハイライト）
            let match_bg = [0.1_f32, 0.5, 0.8, 1.0];
            // ヒントラベル前景色（白）
            let hint_fg = [1.0_f32, 1.0, 1.0, 1.0];
            // ヒントラベル背景色（濃い黄色）
            let hint_bg = [0.8_f32, 0.6, 0.0, 1.0];

            for hint in &qs.hints {
                if hint.row >= grid_rows {
                    continue;
                }

                // マッチ範囲のセルを背景色変更でハイライト
                for col in hint.start_col..hint.end_col.min(grid_cols) {
                    let cell_index = hint.row * grid_cols + col;
                    // 既存セル頂点の fg を維持しつつ bg を変更するため、
                    // 背景だけを上書きする。グリフなしの背景セルとして描画する。
                    let vertex = CellVertex {
                        bg: match_bg,
                        fg: hint_fg,
                        grid_pos: [col as f32, hint.row as f32],
                        uv: [0.0; 4],
                        glyph_offset: [0.0; 2],
                        glyph_size: [0.0; 2],
                        cell_width_scale: 1.0,
                        is_color_glyph: 0.0,
                    };
                    ws.cell_pipeline.overwrite_cell(&ws.gpu.queue, cell_index, &vertex);
                }

                // ヒントラベルをマッチ先頭セルに上書き描画
                let mut col = hint.start_col;
                for ch in hint.label.chars() {
                    if col >= grid_cols {
                        break;
                    }
                    let cell_width_count = char_cell_width(ch);

                    let (uv, glyph_offset, glyph_size) =
                        if let Some(entry) = self.font_ctx.rasterize_glyph(ch, &mut ws.atlas) {
                            let r = entry.region;
                            let uv = [
                                r.x as f32 / atlas_size,
                                r.y as f32 / atlas_size,
                                (r.x + r.width) as f32 / atlas_size,
                                (r.y + r.height) as f32 / atlas_size,
                            ];
                            (
                                uv,
                                [entry.placement_left as f32, entry.placement_top as f32],
                                [r.width as f32, r.height as f32],
                            )
                        } else {
                            ([0.0_f32; 4], [0.0_f32; 2], [0.0_f32; 2])
                        };

                    let vertex = CellVertex {
                        bg: hint_bg,
                        fg: hint_fg,
                        grid_pos: [col as f32, hint.row as f32],
                        uv,
                        glyph_offset,
                        glyph_size,
                        cell_width_scale: if cell_width_count == 2 { 2.0 } else { 1.0 },
                        is_color_glyph: 0.0,
                    };
                    let cell_index = hint.row * grid_cols + col;
                    ws.cell_pipeline.overwrite_cell(&ws.gpu.queue, cell_index, &vertex);
                    col += cell_width_count;
                }
            }

            ws.atlas.upload_if_dirty(&ws.gpu.queue);
        }

        // vi モードカーソル描画
        if let Some(ref vi) = self.vi_mode {
            // vi カーソルのビューポート相対行を計算
            // display_offset=0 → Line(0) がビューポート先頭
            // display_offset>0 → ビューポート先頭は Line(-display_offset)
            let session_ref = self.session_mgr.get(session_id);
            let display_offset = if let Some(s) = session_ref {
                let state = s.term_state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                state.terminal.grid().display_offset()
            } else {
                0
            };

            // vi カーソルの viewport 行 = line.0 + display_offset
            #[allow(clippy::cast_possible_wrap)]
            let vi_viewport_row_i32 = vi.cursor.point.line.0 + display_offset as i32;
            if vi_viewport_row_i32 >= 0 {
                let vi_viewport_row = vi_viewport_row_i32 as usize;
                if vi_viewport_row < grid_rows {
                    let vi_col = vi.cursor.point.column.0.min(grid_cols.saturating_sub(1));
                    let cell_index = vi_viewport_row * grid_cols + vi_col;

                    // 反転色でブロックカーソルを描画（fg/bg 反転）
                    let vi_fg = self.colors.background; // 元の bg を fg に
                    let vi_bg = self.colors.foreground; // 元の fg を bg に

                    let vertex = CellVertex {
                        bg: vi_bg,
                        fg: vi_fg,
                        grid_pos: [vi_col as f32, vi_viewport_row as f32],
                        uv: [0.0; 4],
                        glyph_offset: [0.0; 2],
                        glyph_size: [0.0; 2],
                        cell_width_scale: 1.0,
                        is_color_glyph: 0.0,
                    };
                    ws.cell_pipeline.overwrite_cell(&ws.gpu.queue, cell_index, &vertex);
                    ws.atlas.upload_if_dirty(&ws.gpu.queue);
                }
            }
        }

        // スクロールバー描画（vi モード後、request_redraw の直前）
        if self.config.scrollbar.enabled {
            let session_ref = self.session_mgr.get(session_id);
            if let Some(s) = session_ref {
                let state = s.term_state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                let grid = state.terminal.grid();
                let history_size = grid.history_size();
                let screen_lines = grid.screen_lines();
                let display_offset = grid.display_offset();
                let cols = grid.columns();

                if history_size > 0 && cols > 0 {
                    // スクロールバー位置計算
                    let total_lines = history_size + screen_lines;
                    let thumb_ratio = (screen_lines as f32 / total_lines as f32).clamp(0.0, 1.0);
                    let scroll_ratio = display_offset as f32 / history_size as f32;
                    // scroll_ratio: 0.0 = 最下部（最新）, 1.0 = 最上部（最古）

                    let scrollbar_col = cols.saturating_sub(1);

                    // thumb_rows: サムが占める行数（最低1行）
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let thumb_rows = ((thumb_ratio * screen_lines as f32).ceil() as usize).max(1);

                    // thumb_top_row: サムの先頭行（0=ビューポート先頭, screen_lines-1=末尾）
                    // scroll_ratio=1.0(最上部)のとき thumb_top=0,
                    // scroll_ratio=0.0(最下部)のとき thumb_top=screen_lines-thumb_rows
                    let available = screen_lines.saturating_sub(thumb_rows);
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let thumb_top =
                        (((1.0 - scroll_ratio) * available as f32).round() as usize).min(available);

                    // トラック色（背景より少し明るく半透明）
                    let bg = self.colors.background;
                    let track_bg = [
                        (bg[0] + 0.08).min(1.0),
                        (bg[1] + 0.08).min(1.0),
                        (bg[2] + 0.08).min(1.0),
                        1.0,
                    ];
                    // サム色（前景色を50%透明度で表現: bg と blend）
                    let fg = self.colors.foreground;
                    let thumb_bg = [
                        bg[0] * 0.5 + fg[0] * 0.5,
                        bg[1] * 0.5 + fg[1] * 0.5,
                        bg[2] * 0.5 + fg[2] * 0.5,
                        1.0,
                    ];

                    // スクロールバーセルを上書き
                    for row in 0..screen_lines {
                        let is_thumb = row >= thumb_top && row < thumb_top + thumb_rows;
                        let cell_bg = if is_thumb { thumb_bg } else { track_bg };
                        let vertex = CellVertex {
                            bg: cell_bg,
                            fg: [0.0; 4],
                            grid_pos: [scrollbar_col as f32, row as f32],
                            uv: [0.0; 4],
                            glyph_offset: [0.0; 2],
                            glyph_size: [0.0; 2],
                            cell_width_scale: 1.0,
                            is_color_glyph: 0.0,
                        };
                        let cell_index = row * cols + scrollbar_col;
                        ws.cell_pipeline.overwrite_cell(&ws.gpu.queue, cell_index, &vertex);
                    }
                    drop(state);
                    ws.atlas.upload_if_dirty(&ws.gpu.queue);
                }
            }
        }

        // 閉じる確認ダイアログ オーバーレイ描画
        // pending_close の情報を先にコピーして借用競合を回避する
        let pending_close_info: Option<(&'static str, bool)> = {
            use crate::app::PendingClose;
            self.pending_close.as_ref().and_then(|pending| match pending {
                PendingClose::Session(_, wid) if *wid == window_id => {
                    Some(("Close session? [y/n]", true))
                }
                PendingClose::Session(_, _) => None,
                PendingClose::Quit => Some(("Quit SDIT? [y/n]", true)),
            })
        };
        if let Some((msg, _)) = pending_close_info {
            if grid_rows > 0 && grid_cols > 0 {
                let atlas_size = ws.atlas.size() as f32;
                // 暗い半透明オーバーレイ背景色
                let overlay_bg = [0.0_f32, 0.0, 0.0, 0.7];
                // 白文字
                let overlay_fg = [1.0_f32, 1.0, 1.0, 1.0];

                // 画面中央行に表示
                let overlay_row = grid_rows / 2;
                let msg_len = msg.chars().count().min(grid_cols);
                let start_col = (grid_cols.saturating_sub(msg_len)) / 2;

                // オーバーレイ行のセルを全て暗くする
                for col in 0..grid_cols {
                    let cell_index = overlay_row * grid_cols + col;
                    let vertex = CellVertex {
                        bg: overlay_bg,
                        fg: overlay_fg,
                        grid_pos: [col as f32, overlay_row as f32],
                        uv: [0.0; 4],
                        glyph_offset: [0.0; 2],
                        glyph_size: [0.0; 2],
                        cell_width_scale: 1.0,
                        is_color_glyph: 0.0,
                    };
                    ws.cell_pipeline.overwrite_cell(&ws.gpu.queue, cell_index, &vertex);
                }

                // メッセージ文字を描画
                for (i, ch) in msg.chars().enumerate().take(msg_len) {
                    let col = start_col + i;
                    if col >= grid_cols {
                        break;
                    }
                    let (uv, glyph_offset, glyph_size) =
                        if let Some(entry) = self.font_ctx.rasterize_glyph(ch, &mut ws.atlas) {
                            let r = entry.region;
                            let uv = [
                                r.x as f32 / atlas_size,
                                r.y as f32 / atlas_size,
                                (r.x + r.width) as f32 / atlas_size,
                                (r.y + r.height) as f32 / atlas_size,
                            ];
                            (
                                uv,
                                [entry.placement_left as f32, entry.placement_top as f32],
                                [r.width as f32, r.height as f32],
                            )
                        } else {
                            ([0.0_f32; 4], [0.0_f32; 2], [0.0_f32; 2])
                        };
                    let vertex = CellVertex {
                        bg: overlay_bg,
                        fg: overlay_fg,
                        grid_pos: [col as f32, overlay_row as f32],
                        uv,
                        glyph_offset,
                        glyph_size,
                        cell_width_scale: 1.0,
                        is_color_glyph: 0.0,
                    };
                    let cell_index = overlay_row * grid_cols + col;
                    ws.cell_pipeline.overwrite_cell(&ws.gpu.queue, cell_index, &vertex);
                }

                ws.atlas.upload_if_dirty(&ws.gpu.queue);
            }
        }

        // コマンドパレット オーバーレイ描画
        if let Some(ref cp) = self.command_palette {
            if grid_rows >= 3 && grid_cols >= 4 {
                let atlas_size = ws.atlas.size() as f32;

                // パレット寸法
                let palette_width = grid_cols.min(60).max(20);
                let start_col = (grid_cols.saturating_sub(palette_width)) / 2;
                let start_row = {
                    let palette_height = 1 + cp
                        .filtered_actions
                        .len()
                        .min(crate::command_palette::MAX_VISIBLE_ITEMS);
                    (grid_rows / 3).min(grid_rows.saturating_sub(palette_height + 1))
                };

                // 色定義
                let input_bg = [0.15_f32, 0.15, 0.18, 1.0];
                let item_bg = [0.08_f32, 0.08, 0.10, 1.0];
                let selected_bg = [0.25_f32, 0.35, 0.55, 1.0];
                let fg = self.colors.foreground;
                let dim_fg = [fg[0] * 0.6, fg[1] * 0.6, fg[2] * 0.6, 1.0];

                // ── 入力行（先頭行）──
                let input_row = start_row;
                let input_text = format!("> {}", cp.input);
                let input_chars: Vec<char> = input_text.chars().collect();

                for col_offset in 0..palette_width {
                    let col = start_col + col_offset;
                    if col >= grid_cols {
                        break;
                    }
                    let ch = input_chars.get(col_offset).copied();
                    let (uv, glyph_offset, glyph_size) = if let Some(c) = ch {
                        if let Some(entry) = self.font_ctx.rasterize_glyph(c, &mut ws.atlas) {
                            let r = entry.region;
                            (
                                [
                                    r.x as f32 / atlas_size,
                                    r.y as f32 / atlas_size,
                                    (r.x + r.width) as f32 / atlas_size,
                                    (r.y + r.height) as f32 / atlas_size,
                                ],
                                [entry.placement_left as f32, entry.placement_top as f32],
                                [r.width as f32, r.height as f32],
                            )
                        } else {
                            ([0.0_f32; 4], [0.0_f32; 2], [0.0_f32; 2])
                        }
                    } else {
                        ([0.0_f32; 4], [0.0_f32; 2], [0.0_f32; 2])
                    };
                    let vertex = CellVertex {
                        bg: input_bg,
                        fg,
                        grid_pos: [col as f32, input_row as f32],
                        uv,
                        glyph_offset,
                        glyph_size,
                        cell_width_scale: 1.0,
                        is_color_glyph: 0.0,
                    };
                    ws.cell_pipeline.overwrite_cell(
                        &ws.gpu.queue,
                        input_row * grid_cols + col,
                        &vertex,
                    );
                }

                // ── 候補リスト ──
                let cp_selected = cp.selected_index;
                let cp_items: Vec<String> = cp
                    .filtered_actions
                    .iter()
                    .take(crate::command_palette::MAX_VISIBLE_ITEMS)
                    .map(|(name, _)| format!("  {name}"))
                    .collect();

                for (item_idx, label) in cp_items.iter().enumerate() {
                    let item_row = start_row + 1 + item_idx;
                    if item_row >= grid_rows {
                        break;
                    }
                    let is_selected = item_idx == cp_selected;
                    let row_bg = if is_selected { selected_bg } else { item_bg };
                    let row_fg = if is_selected { [1.0_f32, 1.0, 1.0, 1.0] } else { dim_fg };
                    let label_chars: Vec<char> = label.chars().collect();

                    for col_offset in 0..palette_width {
                        let col = start_col + col_offset;
                        if col >= grid_cols {
                            break;
                        }
                        let ch = label_chars.get(col_offset).copied();
                        let (uv, glyph_offset, glyph_size) = if let Some(c) = ch {
                            if let Some(entry) = self.font_ctx.rasterize_glyph(c, &mut ws.atlas) {
                                let r = entry.region;
                                (
                                    [
                                        r.x as f32 / atlas_size,
                                        r.y as f32 / atlas_size,
                                        (r.x + r.width) as f32 / atlas_size,
                                        (r.y + r.height) as f32 / atlas_size,
                                    ],
                                    [entry.placement_left as f32, entry.placement_top as f32],
                                    [r.width as f32, r.height as f32],
                                )
                            } else {
                                ([0.0_f32; 4], [0.0_f32; 2], [0.0_f32; 2])
                            }
                        } else {
                            ([0.0_f32; 4], [0.0_f32; 2], [0.0_f32; 2])
                        };
                        let vertex = CellVertex {
                            bg: row_bg,
                            fg: row_fg,
                            grid_pos: [col as f32, item_row as f32],
                            uv,
                            glyph_offset,
                            glyph_size,
                            cell_width_scale: 1.0,
                            is_color_glyph: 0.0,
                        };
                        ws.cell_pipeline.overwrite_cell(
                            &ws.gpu.queue,
                            item_row * grid_cols + col,
                            &vertex,
                        );
                    }
                }

                ws.atlas.upload_if_dirty(&ws.gpu.queue);
            }
        }

        ws.window.request_redraw();
    }

    /// ウィンドウリサイズ時に GPU・Terminal を更新する。
    ///
    /// 全セッションの Terminal と PTY をリサイズする。
    /// グリッドサイズが変わるため vi モードのカーソル座標が無効になる可能性があり、
    /// リサイズ時に vi_mode と selection をリセットする。
    pub(crate) fn handle_resize(&mut self, window_id: WindowId, width: u32, height: u32) {
        let Some(ws) = self.windows.get_mut(&window_id) else { return };
        ws.gpu.resize(width, height);

        // 背景画像のユニフォームをリサイズに合わせて更新
        if let Some(bg) = &ws.bg_pipeline {
            use sdit_core::config::BackgroundImageFit;
            let fit_mode = match self.config.window.background_image_fit {
                BackgroundImageFit::Contain => 0,
                BackgroundImageFit::Cover => 1,
                BackgroundImageFit::Fill => 2,
            };
            let opacity = self.config.window.clamped_background_image_opacity();
            bg.update_surface_size(&ws.gpu.queue, [width as f32, height as f32], fit_mode, opacity);
        }

        // グリッドリサイズで vi カーソル座標が無効になるためリセット
        if self.vi_mode.is_some() {
            self.vi_mode = None;
            self.selection = None;
        }

        let metrics = *self.font_ctx.metrics();
        let sidebar_w = ws.sidebar.width_px(metrics.cell_width);
        let padding_x = f32::from(self.config.window.clamped_padding_x());
        let padding_y = f32::from(self.config.window.clamped_padding_y());
        let term_width = (width as f32 - sidebar_w - 2.0 * padding_x).max(0.0);
        let term_height = (height as f32 - 2.0 * padding_y).max(0.0);
        let (cols, rows) =
            calc_grid_size(term_width, term_height, metrics.cell_width, metrics.cell_height);

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

/// hex カラー文字列を `[f32; 4]` (RGBA) に変換する。
///
/// `"#rrggbb"` 形式のみサポート。パース失敗時は `None` を返す。
pub(crate) fn parse_hex_color(hex: &str) -> Option<[f32; 4]> {
    let hex = hex.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some([f32::from(r) / 255.0, f32::from(g) / 255.0, f32::from(b) / 255.0, 1.0])
}

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

    // -----------------------------------------------------------------------
    // parse_hex_color テスト
    // -----------------------------------------------------------------------

    use super::parse_hex_color;

    #[test]
    fn parse_hex_color_valid() {
        let color = parse_hex_color("#ff6600").unwrap();
        assert!((color[0] - 1.0).abs() < 0.01, "r = {}", color[0]);
        assert!((color[1] - 0.4).abs() < 0.01, "g = {}", color[1]);
        assert!((color[2] - 0.0).abs() < 0.01, "b = {}", color[2]);
        assert!((color[3] - 1.0).abs() < 0.01, "a = {}", color[3]);
    }

    #[test]
    fn parse_hex_color_black_white() {
        let black = parse_hex_color("#000000").unwrap();
        for (i, &v) in black.iter().enumerate() {
            let expected = if i == 3 { 1.0_f32 } else { 0.0_f32 };
            assert!((v - expected).abs() < f32::EPSILON, "black[{i}] = {v}");
        }
        let white = parse_hex_color("#ffffff").unwrap();
        assert!((white[0] - 1.0).abs() < f32::EPSILON);
        assert!((white[1] - 1.0).abs() < f32::EPSILON);
        assert!((white[2] - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn parse_hex_color_invalid() {
        // # なし
        assert!(parse_hex_color("ff6600").is_none());
        // 長さ不足
        assert!(parse_hex_color("#ff66").is_none());
        // 無効な文字
        assert!(parse_hex_color("#zzzzzz").is_none());
        // 空文字
        assert!(parse_hex_color("").is_none());
    }
}
