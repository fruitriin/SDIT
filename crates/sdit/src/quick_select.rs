//! QuickSelect モード: キーボードショートカットで画面上のパターンをクリップボードにコピーする。

use sdit_core::grid::Dimensions;
use winit::keyboard::NamedKey;
use winit::window::WindowId;

use crate::app::{QuickSelectHint, QuickSelectState, SditApp};

impl SditApp {
    /// QuickSelect モード中のキー入力を処理する。
    ///
    /// キーを消費した場合は `true` を返す（呼び出し側で `return` すること）。
    pub(crate) fn handle_quick_select_key(
        &mut self,
        logical_key: &winit::keyboard::Key,
        window_id: WindowId,
    ) -> bool {
        use winit::keyboard::Key;

        // Escape で QuickSelect を終了
        if matches!(logical_key, Key::Named(NamedKey::Escape)) {
            self.quick_select = None;
            if let Some(ws) = self.windows.get(&window_id) {
                let sid = ws.active_session_id();
                self.redraw_session(sid);
            }
            return true;
        }

        // Cmd/Ctrl 修飾がある場合は通常処理へ fallthrough しない（ここでは無視して true 返す）
        if self.modifiers.super_key() || self.modifiers.control_key() {
            return true;
        }

        if let Key::Character(s) = logical_key {
            // ヒント文字列を入力
            if let Some(ref mut qs) = self.quick_select {
                qs.input.push_str(s.as_str());
                let input = qs.input.clone();
                // 完全マッチするヒントを探す
                let matched = qs.hints.iter().find(|h| h.label == input).cloned();
                if let Some(hint) = matched {
                    // クリップボードにコピー
                    let copy_text = if self.config.selection.trim_trailing_spaces {
                        crate::cwd_utils::trim_trailing_whitespace(&hint.text)
                    } else {
                        hint.text.clone()
                    };
                    if let Some(cb) = &mut self.clipboard {
                        if let Err(e) = cb.set_text(&copy_text) {
                            log::warn!("QuickSelect clipboard set_text failed: {e}");
                        }
                    }
                    log::info!("QuickSelect: copied {} bytes", copy_text.len());
                    self.quick_select = None;
                } else {
                    // 候補が残っているか確認（前方一致で候補があれば継続、なければ終了）
                    let has_candidate = self
                        .quick_select
                        .as_ref()
                        .is_some_and(|qs| qs.hints.iter().any(|h| h.label.starts_with(&qs.input)));
                    if !has_candidate {
                        self.quick_select = None;
                    }
                }
            }
            if let Some(ws) = self.windows.get(&window_id) {
                let sid = ws.active_session_id();
                self.redraw_session(sid);
            }
            return true;
        }

        // 他のキーは無視（消費する）
        true
    }

    /// QuickSelect アクションを処理する（モード起動/終了トグル）。
    pub(crate) fn handle_quick_select_action(&mut self, window_id: WindowId) {
        // トグル: モード中なら終了
        if self.quick_select.is_some() {
            self.quick_select = None;
            if let Some(ws) = self.windows.get(&window_id) {
                let sid = ws.active_session_id();
                self.redraw_session(sid);
            }
            return;
        }

        let Some(ws) = self.windows.get(&window_id) else { return };
        let sid = ws.active_session_id();
        let Some(session) = self.session_mgr.get(sid) else { return };

        // デフォルトパターン + コンパイル済みカスタムパターンを結合
        let default_patterns = sdit_core::terminal::url_detector::default_quick_select_patterns();
        let mut all_patterns: Vec<_> = default_patterns.iter().collect();
        all_patterns.extend(self.compiled_quick_select_patterns.iter());

        // 全ビューポート行をスキャンしてヒントを生成
        let mut hints: Vec<QuickSelectHint> = Vec::new();
        {
            use sdit_core::index::{Column, Line, Point};
            use sdit_core::terminal::url_detector::detect_patterns_in_line;

            let state =
                session.term_state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let grid = state.terminal.grid();
            let rows = grid.screen_lines();
            let cols = grid.columns();

            for row in 0..rows {
                let line_cells: Vec<_> = (0..cols)
                    .map(|c| {
                        #[allow(clippy::cast_possible_wrap)]
                        grid[Point::new(Line(row as i32), Column(c))].clone()
                    })
                    .collect();

                let matches = detect_patterns_in_line(&line_cells, &all_patterns);
                for pm in matches {
                    let label = QuickSelectState::generate_label(hints.len());
                    hints.push(QuickSelectHint {
                        label,
                        row,
                        start_col: pm.start_col,
                        end_col: pm.end_col,
                        text: pm.text,
                    });
                }
            }
        }

        if hints.is_empty() {
            log::info!("QuickSelect: no patterns found on screen");
            return;
        }

        self.quick_select = Some(QuickSelectState { hints, input: String::new() });
        self.redraw_session(sid);
    }
}
