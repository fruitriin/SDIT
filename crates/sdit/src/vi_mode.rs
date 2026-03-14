//! vi モード（コピーモード）のキー処理。
//!
//! `handle_vi_mode_key` がエントリーポイント。Escape/v/V/y/hjkl/w/b/e/0/$/{/}/H/M/L/G/g/n/N
//! などのキーを処理し、カーソル移動・選択・ヤンクを行う。

use sdit_core::grid::{Dimensions, Scroll};
use sdit_core::selection::{Selection, SelectionType};
use sdit_core::terminal::vi_mode::{ViCursor, ViMotion};
use winit::keyboard::{Key, NamedKey};
use winit::window::WindowId;

use crate::app::{SditApp, SearchState, ViModeState, ViSelectionKind};

impl SditApp {
    /// vi モード（コピーモード）をトグルする。
    ///
    /// 非アクティブ → アクティブ: カーソルを現在のターミナルカーソル位置に合わせる。
    /// アクティブ → 非アクティブ: 選択をクリアしてライブビューポートに戻る。
    pub(crate) fn toggle_vi_mode(&mut self, window_id: WindowId) {
        if self.vi_mode.is_some() {
            // 終了
            self.vi_mode = None;
            self.selection = None;
            if let Some(ws) = self.windows.get(&window_id) {
                let sid = ws.active_session_id();
                self.redraw_session(sid);
            }
        } else {
            // 起動: カーソルをターミナルカーソル位置に初期化
            if let Some(ws) = self.windows.get(&window_id) {
                let sid = ws.active_session_id();
                if let Some(session) = self.session_mgr.get(sid) {
                    let state = session
                        .term_state
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    let cursor_point = state.terminal.grid().cursor.point;
                    drop(state);
                    self.vi_mode = Some(ViModeState {
                        cursor: ViCursor::new(cursor_point),
                        selection: None,
                        selection_origin: None,
                        pending_g: false,
                    });
                    self.redraw_session(sid);
                }
            }
        }
    }

    /// vi モード中のキー入力を処理する。
    ///
    /// キーを消費した場合は `true` を返す（呼び出し側で `return` すること）。
    #[allow(clippy::too_many_lines)]
    pub(crate) fn handle_vi_mode_key(&mut self, logical_key: &Key, window_id: WindowId) -> bool {
        // Escape: vi モード終了 + 選択クリア
        if matches!(logical_key, Key::Named(NamedKey::Escape)) {
            self.vi_mode = None;
            self.selection = None;
            if let Some(ws) = self.windows.get(&window_id) {
                let sid = ws.active_session_id();
                self.redraw_session(sid);
            }
            return true;
        }

        // 文字キー処理
        if let Key::Character(s) = logical_key {
            let ch = s.chars().next().unwrap_or('\0');

            // g キーのダブル入力: gg = Top
            if ch == 'g' {
                let pending = self.vi_mode.as_ref().is_some_and(|vi| vi.pending_g);
                if pending {
                    // gg: 最上行へ
                    if let Some(vi) = &mut self.vi_mode {
                        vi.pending_g = false;
                    }
                    self.vi_apply_motion(ViMotion::Top, window_id);
                } else if let Some(vi) = &mut self.vi_mode {
                    vi.pending_g = true;
                }
                return true;
            }

            // g 以外のキーで pending_g をリセット
            if let Some(vi) = &mut self.vi_mode {
                vi.pending_g = false;
            }

            match ch {
                // --- カーソル移動 ---
                'h' => self.vi_apply_motion(ViMotion::Left, window_id),
                'j' => self.vi_apply_motion(ViMotion::Down, window_id),
                'k' => self.vi_apply_motion(ViMotion::Up, window_id),
                'l' => self.vi_apply_motion(ViMotion::Right, window_id),
                'w' => self.vi_apply_motion(ViMotion::WordRight, window_id),
                'b' => self.vi_apply_motion(ViMotion::WordLeft, window_id),
                'e' => self.vi_apply_motion(ViMotion::WordEnd, window_id),
                '0' => self.vi_apply_motion(ViMotion::First, window_id),
                '$' => self.vi_apply_motion(ViMotion::Last, window_id),
                '{' => self.vi_apply_motion(ViMotion::ParagraphUp, window_id),
                '}' => self.vi_apply_motion(ViMotion::ParagraphDown, window_id),
                'H' => self.vi_apply_motion(ViMotion::ScreenTop, window_id),
                'M' => self.vi_apply_motion(ViMotion::ScreenMiddle, window_id),
                'L' => self.vi_apply_motion(ViMotion::ScreenBottom, window_id),
                'G' => self.vi_apply_motion(ViMotion::Bottom, window_id),

                // --- 選択 ---
                'v' => {
                    let current_point =
                        self.vi_mode.as_ref().map(|vi| vi.cursor.point).unwrap_or_default();
                    if let Some(vi) = &mut self.vi_mode {
                        if vi.selection == Some(ViSelectionKind::Char) {
                            // トグル: 選択解除
                            vi.selection = None;
                            vi.selection_origin = None;
                            self.selection = None;
                        } else {
                            vi.selection = Some(ViSelectionKind::Char);
                            vi.selection_origin = Some(current_point);
                        }
                    }
                    if let Some(ws) = self.windows.get(&window_id) {
                        let sid = ws.active_session_id();
                        self.redraw_session(sid);
                    }
                }
                'V' => {
                    let current_point =
                        self.vi_mode.as_ref().map(|vi| vi.cursor.point).unwrap_or_default();
                    if let Some(vi) = &mut self.vi_mode {
                        if vi.selection == Some(ViSelectionKind::Line) {
                            vi.selection = None;
                            vi.selection_origin = None;
                            self.selection = None;
                        } else {
                            vi.selection = Some(ViSelectionKind::Line);
                            vi.selection_origin = Some(current_point);
                        }
                    }
                    if let Some(ws) = self.windows.get(&window_id) {
                        let sid = ws.active_session_id();
                        self.redraw_session(sid);
                    }
                }

                // --- ヤンク ---
                'y' => {
                    self.vi_yank(window_id);
                }

                // --- 検索 ---
                '/' => {
                    // 検索モード起動
                    self.search = Some(SearchState::new());
                    if let Some(ws) = self.windows.get(&window_id) {
                        let sid = ws.active_session_id();
                        self.redraw_session(sid);
                    }
                }
                'n' => {
                    self.search_navigate(1, window_id);
                }
                'N' => {
                    self.search_navigate(-1, window_id);
                }

                _ => {} // 未割り当てキーは消費するが何もしない
            }
        } else {
            // Named key: Arrow キーをモーションにマップ
            match logical_key {
                Key::Named(NamedKey::ArrowUp) => self.vi_apply_motion(ViMotion::Up, window_id),
                Key::Named(NamedKey::ArrowDown) => self.vi_apply_motion(ViMotion::Down, window_id),
                Key::Named(NamedKey::ArrowLeft) => self.vi_apply_motion(ViMotion::Left, window_id),
                Key::Named(NamedKey::ArrowRight) => {
                    self.vi_apply_motion(ViMotion::Right, window_id);
                }
                _ => {} // 他のキーは消費するが何もしない
            }
        }

        true // vi モード中は全キーを消費する
    }

    /// vi カーソルにモーションを適用し、選択を更新、スクロール追従、再描画を行う。
    fn vi_apply_motion(&mut self, motion: ViMotion, window_id: WindowId) {
        let Some(ws) = self.windows.get(&window_id) else { return };
        let sid = ws.active_session_id();
        let Some(session) = self.session_mgr.get(sid) else { return };

        let new_cursor = {
            let state =
                session.term_state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let grid = state.terminal.grid();
            if let Some(vi) = &self.vi_mode {
                vi.cursor.motion(grid, motion)
            } else {
                return;
            }
        };

        // カーソル更新
        if let Some(vi) = &mut self.vi_mode {
            vi.cursor = new_cursor;
        }

        // 選択を更新
        self.vi_update_selection(window_id);

        // スクロール追従: カーソルがビューポート外ならスクロール
        self.vi_scroll_to_cursor(window_id);

        self.redraw_session(sid);
    }

    /// vi カーソル位置に基づいて selection を更新する。
    ///
    /// `origin` と `cursor_point` はグリッド座標（Line(0) = ビューポート先頭、
    /// 負値 = スクロールバック履歴）なので、display_offset による変換は行わない。
    /// viewport 変換は描画時（`redraw_session`）のみ行う。
    fn vi_update_selection(&mut self, window_id: WindowId) {
        let Some(ws) = self.windows.get(&window_id) else { return };
        let _sid = ws.active_session_id();

        let Some(vi) = &self.vi_mode else { return };
        let Some(sel_kind) = vi.selection else {
            self.selection = None;
            return;
        };
        let Some(origin) = vi.selection_origin else {
            self.selection = None;
            return;
        };

        let cursor_point = vi.cursor.point;

        let sel_type = match sel_kind {
            ViSelectionKind::Char | ViSelectionKind::Block => SelectionType::Simple,
            ViSelectionKind::Line => SelectionType::Lines,
        };

        // origin と cursor_point はグリッド座標のままで Selection に渡す。
        // display_offset による viewport 変換は描画時にのみ行う。
        let mut sel = Selection::new(sel_type, origin);
        sel.end = cursor_point;
        self.selection = Some(sel);
    }

    /// vi カーソルがビューポート外にある場合、スクロールして表示範囲内に収める。
    fn vi_scroll_to_cursor(&mut self, window_id: WindowId) {
        let Some(ws) = self.windows.get(&window_id) else { return };
        let sid = ws.active_session_id();
        let Some(session) = self.session_mgr.get(sid) else { return };
        let Some(vi) = &self.vi_mode else { return };

        let cursor_line = vi.cursor.point.line.0;

        let mut state =
            session.term_state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let display_offset = state.terminal.grid().display_offset();
        let screen_lines = state.terminal.grid().screen_lines();

        // 現在の表示範囲: Line(-display_offset) .. Line(screen_lines - 1 - display_offset)
        // cursor_line が表示範囲外ならスクロール
        #[allow(clippy::cast_possible_wrap)]
        let view_top = -(display_offset as i32);
        #[allow(clippy::cast_possible_wrap)]
        let view_bottom = (screen_lines as i32) - 1 - (display_offset as i32);

        if cursor_line < view_top {
            // カーソルが上にはみ出た → 上にスクロール
            let delta = view_top - cursor_line;
            #[allow(clippy::cast_possible_wrap)]
            state.terminal.grid_mut().scroll_display(Scroll::Delta(delta as isize));
        } else if cursor_line > view_bottom {
            // カーソルが下にはみ出た → 下にスクロール
            let delta = cursor_line - view_bottom;
            #[allow(clippy::cast_possible_wrap)]
            state.terminal.grid_mut().scroll_display(Scroll::Delta(-(delta as isize)));
        }
    }

    /// vi モードでヤンク（選択テキストをクリップボードにコピー）し、vi モードを終了する。
    fn vi_yank(&mut self, window_id: WindowId) {
        let Some(ws) = self.windows.get(&window_id) else { return };
        let sid = ws.active_session_id();
        let Some(session) = self.session_mgr.get(sid) else { return };

        if let Some(ref sel) = self.selection {
            let state =
                session.term_state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let text = sdit_core::selection::selected_text(state.terminal.grid(), sel);
            drop(state);
            if !text.is_empty() {
                if let Some(cb) = &mut self.clipboard {
                    if let Err(e) = cb.set_text(&text) {
                        log::warn!("vi yank clipboard set_text failed: {e}");
                    } else {
                        log::info!("vi yank: copied {} bytes", text.len());
                    }
                }
            }
        }

        // vi モード終了 + 選択クリア
        self.vi_mode = None;
        self.selection = None;
        if let Some(ws) = self.windows.get(&window_id) {
            let sid = ws.active_session_id();
            self.redraw_session(sid);
        }
    }
}
