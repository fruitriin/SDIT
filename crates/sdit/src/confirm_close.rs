use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;

use sdit_core::config::ConfirmClose;

use crate::app::{PendingClose, SditApp};

impl SditApp {
    /// 指定セッションを閉じるときに確認ダイアログを表示すべきかどうかを返す。
    pub(crate) fn should_confirm_close(&self, session_id: sdit_core::session::SessionId) -> bool {
        match self.config.window.confirm_close {
            ConfirmClose::Never => false,
            ConfirmClose::Always => true,
            ConfirmClose::ProcessRunning => self
                .session_mgr
                .get(session_id)
                .map(|s| s.has_foreground_process())
                .unwrap_or(false),
        }
    }

    /// Quit 時に確認ダイアログを表示すべきかどうかを返す。
    ///
    /// いずれかのセッションでフォアグラウンドプロセスが実行中なら確認する。
    pub(crate) fn should_confirm_quit(&self) -> bool {
        match self.config.window.confirm_close {
            ConfirmClose::Never => false,
            ConfirmClose::Always => !self.windows.is_empty(),
            ConfirmClose::ProcessRunning => {
                self.session_mgr.all().any(|s| s.has_foreground_process())
            }
        }
    }

    /// `pending_close` を実行する（y/Enter 押下時）。
    pub(crate) fn execute_pending_close(&mut self, event_loop: &ActiveEventLoop) {
        let Some(pending) = self.pending_close.take() else { return };
        match pending {
            PendingClose::Session(_, window_id) => {
                let window_closed = self.remove_active_session(window_id);
                if window_closed && self.windows.is_empty() {
                    event_loop.exit();
                }
            }
            PendingClose::Quit => {
                let window_ids: Vec<WindowId> = self.windows.keys().copied().collect();
                for wid in window_ids {
                    self.close_window(wid);
                }
                event_loop.exit();
            }
        }
    }

    /// `pending_close` をキャンセルする（n/Escape 押下時）。
    pub(crate) fn cancel_pending_close(&mut self, window_id: WindowId) {
        self.pending_close = None;
        if let Some(ws) = self.windows.get(&window_id) {
            let sid = ws.active_session_id();
            self.redraw_session(sid);
        }
    }
}
