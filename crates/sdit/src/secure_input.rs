//! Secure Keyboard Entry（macOS）の管理。
//!
//! `EnableSecureEventInput` / `DisableSecureEventInput` API を安全にラップする。
//! `SditApp` のフォーカスイベントや `ToggleSecureInput` アクションから呼び出される。

use crate::app::SditApp;

// ---------------------------------------------------------------------------
// macOS Secure Event Input API ラッパー
// ---------------------------------------------------------------------------

/// macOS の Secure Event Input API。
///
/// Security.framework に含まれる関数群で、キーストロークを他のプロセスから
/// キャプチャされないように保護する。
/// `unsafe_code = "deny"` の制約のため、このモジュール内に unsafe を限定する。
#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
mod sys {
    unsafe extern "C" {
        pub fn EnableSecureEventInput();
        pub fn DisableSecureEventInput();
        pub fn IsSecureEventInputEnabled() -> bool;
    }

    /// 現在 Secure Event Input が有効かどうかを返す。
    pub fn is_enabled() -> bool {
        // SAFETY: macOS Security.framework の公開 C API 呼び出し。
        // スレッドセーフかつ副作用なし。
        unsafe { IsSecureEventInputEnabled() }
    }

    /// Secure Event Input を有効化する。
    pub fn enable() {
        // SAFETY: macOS Security.framework の公開 C API 呼び出し。
        // アプリケーションの有効期間中に呼ぶことを前提とする。
        unsafe { EnableSecureEventInput() }
    }

    /// Secure Event Input を無効化する。
    pub fn disable() {
        // SAFETY: macOS Security.framework の公開 C API 呼び出し。
        unsafe { DisableSecureEventInput() }
    }
}

// ---------------------------------------------------------------------------
// SditApp メソッド
// ---------------------------------------------------------------------------

impl SditApp {
    /// Secure Keyboard Entry の有効/無効をトグルする（macOS のみ）。
    #[cfg(target_os = "macos")]
    pub(crate) fn toggle_secure_input(&mut self) {
        // 安全のため、現在の API 状態と自分のフィールドを同期する
        let currently_enabled = sys::is_enabled();
        if currently_enabled {
            sys::disable();
            self.secure_input_enabled = false;
            log::info!("Secure Keyboard Entry を無効化しました");
        } else {
            sys::enable();
            self.secure_input_enabled = true;
            log::info!("Secure Keyboard Entry を有効化しました");
        }
    }

    /// フォーカス取得時に auto_secure_input が有効なら Secure Input を有効化する（macOS のみ）。
    #[cfg(target_os = "macos")]
    pub(crate) fn enable_secure_input_if_configured(&mut self) {
        if self.config.security.auto_secure_input && !self.secure_input_enabled {
            sys::enable();
            self.secure_input_enabled = true;
            log::info!("auto_secure_input: Secure Keyboard Entry を有効化しました");
        }
    }

    /// フォーカス喪失時に auto_secure_input で有効化した Secure Input を無効化する（macOS のみ）。
    #[cfg(target_os = "macos")]
    pub(crate) fn disable_secure_input_if_auto(&mut self) {
        if self.config.security.auto_secure_input && self.secure_input_enabled {
            sys::disable();
            self.secure_input_enabled = false;
            log::info!(
                "auto_secure_input: Secure Keyboard Entry を無効化しました（フォーカス喪失）"
            );
        }
    }
}
