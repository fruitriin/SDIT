//! セッションリネーム機能。
//!
//! サイドバー上でのダブルクリックによるリネームモード開始と、
//! リネームモード中のキー入力処理（Enter 確定 / Escape キャンセル / Backspace 削除 / 文字入力）。

use winit::keyboard::{Key, NamedKey};
use winit::window::WindowId;

use crate::app::SditApp;

impl SditApp {
    /// サイドバー行クリック処理（シングル / ダブルクリック判定）。
    ///
    /// ダブルクリック（500ms 以内の同一行への2回目のクリック）でリネームモードに移行する。
    /// シングルクリックではセッション切替とドラッグ開始の記録のみ行う。
    pub(crate) fn handle_sidebar_click(&mut self, window_id: WindowId, row: usize) {
        let now = std::time::Instant::now();
        let is_double_click = self.last_click_time.map_or(false, |t| {
            now.duration_since(t).as_millis() < 500 && self.last_click_pos == Some((row, 0))
        });

        if is_double_click {
            let sid = self.windows.get(&window_id).and_then(|ws| ws.sessions.get(row).copied());
            if let Some(sid) = sid {
                let current_name = self
                    .session_mgr
                    .get(sid)
                    .and_then(|s| s.custom_name.clone())
                    .unwrap_or_default();
                self.renaming_session = Some((sid, current_name));
                self.last_click_time = None;
                self.last_click_pos = None;
                if let Some(ws) = self.windows.get_mut(&window_id) {
                    ws.window.request_redraw();
                }
            }
        } else {
            self.last_click_time = Some(now);
            self.last_click_pos = Some((row, 0));
            self.drag_source_row = Some(row);
            if let Some(ws) = self.windows.get(&window_id) {
                if row != ws.active_index {
                    let ws = self.windows.get_mut(&window_id).unwrap();
                    ws.active_index = row;
                    let sid = ws.active_session_id();
                    self.redraw_session(sid);
                }
            }
        }
    }

    /// リネームモード中のキー入力を処理する。
    ///
    /// 処理を消費した場合は `true` を返す（呼び出し元が `return` する）。
    ///
    /// - `Enter`: 入力テキストを確定してリネームモードを終了
    /// - `Escape`: キャンセルしてリネームモードを終了
    /// - `Backspace`: 最後の文字を削除
    /// - 通常文字（Cmd/Ctrl なし）: テキストに追加（最大 256 文字）
    /// - その他: 無視してリネームモードを維持（true を返す）
    pub(crate) fn handle_rename_key(&mut self, key: &Key, window_id: WindowId) -> bool {
        match key {
            Key::Named(NamedKey::Enter) => {
                if let Some((sid, text)) = self.renaming_session.take() {
                    let name = text.trim().to_owned();
                    if let Some(session) = self.session_mgr.get_mut(sid) {
                        session.custom_name = if name.is_empty() { None } else { Some(name) };
                    }
                }
                if let Some(ws) = self.windows.get(&window_id) {
                    let sid = ws.active_session_id();
                    self.redraw_session(sid);
                }
                true
            }
            Key::Named(NamedKey::Escape) => {
                self.renaming_session = None;
                if let Some(ws) = self.windows.get(&window_id) {
                    let sid = ws.active_session_id();
                    self.redraw_session(sid);
                }
                true
            }
            Key::Named(NamedKey::Backspace) => {
                if let Some((_, ref mut text)) = self.renaming_session {
                    text.pop();
                }
                if let Some(ws) = self.windows.get(&window_id) {
                    let sid = ws.active_session_id();
                    self.redraw_session(sid);
                }
                true
            }
            Key::Character(s) => {
                if !self.modifiers.super_key() && !self.modifiers.control_key() {
                    if let Some((_, ref mut text)) = self.renaming_session {
                        if text.chars().count() + s.chars().count() <= 256 {
                            text.push_str(s.as_str());
                        }
                    }
                    if let Some(ws) = self.windows.get(&window_id) {
                        let sid = ws.active_session_id();
                        self.redraw_session(sid);
                    }
                    true
                } else {
                    false
                }
            }
            _ => true, // その他のキーは無視してリネームモードを維持
        }
    }
}
