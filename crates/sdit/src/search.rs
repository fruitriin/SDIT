use winit::window::WindowId;

use crate::app::SditApp;

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
