use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;

use sdit_core::config::keybinds::Action;
use sdit_core::terminal::TermMode;

use crate::app::{
    PendingClose, SditApp, SearchState, confirm_unsafe_paste, is_unsafe_paste, wrap_bracketed_paste,
};
use crate::cwd_utils::trim_trailing_whitespace;

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
                self.detach_session_to_new_window(window_id, event_loop, None);
            }
            Action::NewWindow => {
                // inherit_working_directory: アクティブセッションの CWD を継承する
                let inherit_cwd = if self.config.window.inherit_working_directory {
                    if let Some(ws) = self.windows.get(&window_id) {
                        let active_sid = ws.active_session_id();
                        self.session_mgr.get(active_sid).and_then(|s| s.cwd.clone())
                    } else {
                        None
                    }
                } else {
                    None
                };
                self.create_window_with_cwd(event_loop, None, inherit_cwd);
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
                if let Some(ws) = self.windows.get(&window_id) {
                    let sid = ws.active_session_id();
                    if self.should_confirm_close(sid) {
                        self.pending_close = Some(PendingClose::Session(sid, window_id));
                        self.redraw_session(sid);
                        return;
                    }
                }
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
                    let raw_text = sdit_core::selection::selected_text(state.terminal.grid(), sel);
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
                                log::warn!("Clipboard set_text failed: {e}");
                            }
                        }
                    }
                }
                // Terminal.app 準拠: コピー後も選択を維持する
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
                // 実行中プロセスがあれば確認ダイアログを表示する。
                if self.should_confirm_quit() {
                    self.pending_close = Some(PendingClose::Quit);
                    // アクティブウィンドウを再描画して確認オーバーレイを表示する
                    if let Some(ws) = self.windows.get(&window_id) {
                        let sid = ws.active_session_id();
                        self.redraw_session(sid);
                    }
                    return;
                }
                // 全ウィンドウを閉じる前にスナップショットを保存する。
                self.save_session_snapshot();
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
            Action::PrevPrompt => {
                let Some(ws) = self.windows.get(&window_id) else { return };
                let Some(session) = self.session_mgr.get(ws.active_session_id()) else {
                    return;
                };
                let sid = ws.active_session_id();
                {
                    let mut state = session
                        .term_state
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    if let Some(target_line) = state.terminal.prev_prompt() {
                        // target_line はビューポート内の行番号（0-based）。
                        // ビューポートの上端を target_line に合わせる:
                        //   新しい display_offset = history - target_line
                        let history = state.terminal.grid().history_size() as isize;
                        let current_offset = state.terminal.grid().display_offset() as isize;
                        let new_offset =
                            (history - target_line as isize).max(0).min(history) as usize;
                        let delta = new_offset as isize - current_offset;
                        state
                            .terminal
                            .grid_mut()
                            .scroll_display(sdit_core::grid::Scroll::Delta(delta));
                    }
                }
                self.redraw_session(sid);
            }
            Action::NextPrompt => {
                let Some(ws) = self.windows.get(&window_id) else { return };
                let Some(session) = self.session_mgr.get(ws.active_session_id()) else {
                    return;
                };
                let sid = ws.active_session_id();
                {
                    let mut state = session
                        .term_state
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    if let Some(target_line) = state.terminal.next_prompt() {
                        let history = state.terminal.grid().history_size() as isize;
                        let current_offset = state.terminal.grid().display_offset() as isize;
                        let new_offset =
                            (history - target_line as isize).max(0).min(history) as usize;
                        let delta = new_offset as isize - current_offset;
                        state
                            .terminal
                            .grid_mut()
                            .scroll_display(sdit_core::grid::Scroll::Delta(delta));
                    }
                }
                self.redraw_session(sid);
            }
            Action::QuickSelect => {
                self.handle_quick_select_action(window_id);
            }
            Action::ToggleViMode => {
                self.toggle_vi_mode(window_id);
            }
            Action::ToggleSecureInput => {
                #[cfg(target_os = "macos")]
                self.toggle_secure_input();
                #[cfg(not(target_os = "macos"))]
                log::info!("ToggleSecureInput は macOS 以外では無視されます");
            }
            Action::NextTheme => {
                self.cycle_theme(true);
                for ws in self.windows.values() {
                    ws.window.request_redraw();
                }
            }
            Action::PreviousTheme => {
                self.cycle_theme(false);
                for ws in self.windows.values() {
                    ws.window.request_redraw();
                }
            }
            Action::ToggleDecorations => {
                use sdit_core::config::Decorations;
                let new_dec = match self.config.window.decorations {
                    Decorations::Full => Decorations::None,
                    Decorations::None => Decorations::Full,
                };
                self.config.window.decorations = new_dec;
                let has_dec = new_dec == Decorations::Full;
                for ws in self.windows.values() {
                    ws.window.set_decorations(has_dec);
                }
                // 設定ファイルに保存する
                let path = sdit_core::config::Config::default_path();
                if let Err(e) = self.config.save(&path) {
                    log::warn!("Failed to save config after ToggleDecorations: {e}");
                }
            }
            Action::ToggleAlwaysOnTop => {
                use winit::window::WindowLevel;
                self.config.window.always_on_top = !self.config.window.always_on_top;
                let level = if self.config.window.always_on_top {
                    WindowLevel::AlwaysOnTop
                } else {
                    WindowLevel::Normal
                };
                for ws in self.windows.values() {
                    ws.window.set_window_level(level);
                }
                // 設定ファイルに保存する
                let path = sdit_core::config::Config::default_path();
                if let Err(e) = self.config.save(&path) {
                    log::warn!("Failed to save config after ToggleAlwaysOnTop: {e}");
                }
            }
            Action::ToggleCommandPalette => {
                if self.command_palette.is_some() {
                    self.command_palette = None;
                } else {
                    self.command_palette = Some(crate::command_palette::CommandPaletteState::new());
                }
                if let Some(ws) = self.windows.get(&window_id) {
                    let sid = ws.active_session_id();
                    self.redraw_session(sid);
                }
            }
            Action::BringToFront => {
                // 全ウィンドウをフォアグラウンドに持ってくる
                for ws in self.windows.values() {
                    ws.window.focus_window();
                }
            }
        }
    }

    /// `performable = true` 時に、アクションが現在実行可能かどうかを返す。
    ///
    /// 実行不可の場合はキーを PTY に転送してアクションをスキップする。
    pub(crate) fn can_perform(&self, action: Action, window_id: WindowId) -> bool {
        match action {
            // 選択テキストが存在する場合のみ実行可能
            Action::Copy => self.selection.is_some(),
            // 検索バーが開いている場合のみ実行可能
            Action::SearchNext | Action::SearchPrev => self.search.is_some(),
            // クリップボードに内容がある場合のみ実行可能
            Action::Paste => self
                .clipboard
                .as_ref()
                .and_then(|cb| {
                    // arboard の get_text は &mut self が必要なため直接チェック不可。
                    // ここでは ClipboardWrite 等で保存した最後の内容は分からないため、
                    // 保守的に「常に実行可能」とする（実際には Paste 後に空チェックする）。
                    // TODO: 将来的に clipboard 内容のキャッシュを保持する
                    let _ = cb;
                    Some(true)
                })
                .unwrap_or(false),
            // 検索バーが開いていてマッチがある場合のみ実行可能
            Action::Search => {
                // ウィンドウにセッションがある場合は実行可能
                self.windows.contains_key(&window_id)
            }
            // その他のアクションは常に実行可能
            _ => true,
        }
    }
}
