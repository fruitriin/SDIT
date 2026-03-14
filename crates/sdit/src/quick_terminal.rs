//! Quick Terminal — グローバルホットキーで画面端からスライドインするターミナル。
//!
//! macOS 固有機能。`config.quick_terminal.enabled = true` のとき初期化される。

use std::time::Instant;

/// screen_size が (0, 0) のときに使うフォールバック幅（物理ピクセル）。
const FALLBACK_SCREEN_WIDTH: u32 = 800;
/// screen_size が (0, 0) のときに使うフォールバック高さ（物理ピクセル）。
const FALLBACK_SCREEN_HEIGHT: u32 = 600;

// ---------------------------------------------------------------------------
// アニメーション方向
// ---------------------------------------------------------------------------

/// スライドアニメーションの方向。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AnimationDirection {
    /// 画面外 → 指定位置（表示）。
    SlideIn,
    /// 指定位置 → 画面外（非表示）。
    SlideOut,
}

// ---------------------------------------------------------------------------
// アニメーション状態
// ---------------------------------------------------------------------------

/// Quick Terminal のスライドアニメーション状態。
pub(crate) struct QuickTerminalAnimation {
    /// アニメーション開始時刻。
    pub(crate) start_time: Instant,
    /// アニメーション時間（秒）。
    pub(crate) duration_secs: f32,
    /// スライド方向。
    pub(crate) direction: AnimationDirection,
}

impl QuickTerminalAnimation {
    /// 現在の進行度（0.0〜1.0）を返す。アニメーションが完了していれば 1.0 を返す。
    pub(crate) fn progress(&self) -> f32 {
        if self.duration_secs <= 0.0 {
            return 1.0;
        }
        let elapsed = self.start_time.elapsed().as_secs_f32();
        (elapsed / self.duration_secs).min(1.0)
    }

    /// Hermite イージング関数（滑らかなスライドアニメーション用）。
    ///
    /// `t * t * (3 - 2t)` の計算式で、加速・減速を伴う補間。
    pub(crate) fn ease_in_out(t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }

    /// アニメーションが完了しているかどうかを返す。
    pub(crate) fn is_complete(&self) -> bool {
        self.progress() >= 1.0
    }
}

// ---------------------------------------------------------------------------
// Quick Terminal 状態管理
// ---------------------------------------------------------------------------

/// Quick Terminal の全体状態。
pub(crate) struct QuickTerminalState {
    /// ドロップダウンが表示中かどうか。
    pub(crate) visible: bool,
    /// 現在のアニメーション状態（アニメーション中のみ Some）。
    pub(crate) animation: Option<QuickTerminalAnimation>,
    /// Quick Terminal ウィンドウの `WindowId`（生成済みのみ Some）。
    pub(crate) window_id: Option<winit::window::WindowId>,
    /// Quick Terminal ウィンドウが生成済みかどうか。
    pub(crate) window_created: bool,
    /// グローバルホットキーマネージャ（macOS のみ）。
    #[cfg(target_os = "macos")]
    pub(crate) hotkey_manager: Option<global_hotkey::GlobalHotKeyManager>,
    /// 登録済みホットキーの ID（macOS のみ）。
    #[cfg(target_os = "macos")]
    pub(crate) hotkey_id: Option<u32>,
}

impl QuickTerminalState {
    /// 新しい空の状態を作成する。
    pub(crate) fn new() -> Self {
        Self {
            visible: false,
            animation: None,
            window_id: None,
            window_created: false,
            #[cfg(target_os = "macos")]
            hotkey_manager: None,
            #[cfg(target_os = "macos")]
            hotkey_id: None,
        }
    }

    /// アニメーションが現在実行中かどうかを返す。
    pub(crate) fn is_animating(&self) -> bool {
        self.animation.as_ref().is_some_and(|a| !a.is_complete())
    }

    /// スライドインアニメーションを開始する。
    pub(crate) fn start_slide_in(&mut self, duration_secs: f32) {
        self.animation = Some(QuickTerminalAnimation {
            start_time: Instant::now(),
            duration_secs,
            direction: AnimationDirection::SlideIn,
        });
        self.visible = true;
    }

    /// スライドアウトアニメーションを開始する。
    pub(crate) fn start_slide_out(&mut self, duration_secs: f32) {
        self.animation = Some(QuickTerminalAnimation {
            start_time: Instant::now(),
            duration_secs,
            direction: AnimationDirection::SlideOut,
        });
    }

    /// アニメーションを完了してクリアする。スライドアウト完了時は visible = false にする。
    pub(crate) fn finish_animation(&mut self) {
        if let Some(anim) = &self.animation {
            if anim.direction == AnimationDirection::SlideOut {
                self.visible = false;
            }
        }
        self.animation = None;
    }
}

// ---------------------------------------------------------------------------
// グローバルホットキー: macOS 固有実装
// ---------------------------------------------------------------------------

/// ホットキー文字列（例: "ctrl+`"）をパースして `HotKey` を生成する。
///
/// 書式: 修飾キー（ctrl/shift/alt/super）をプラスで繋ぎ、最後にキー名を指定する。
/// 例: "ctrl+`", "ctrl+shift+t", "super+`"
///
/// パースに失敗した場合は `None` を返す。
#[cfg(target_os = "macos")]
pub(crate) fn parse_hotkey(hotkey_str: &str) -> Option<global_hotkey::hotkey::HotKey> {
    use global_hotkey::hotkey::{Code, HotKey, Modifiers};

    // DoS 防止: 文字列長の上限チェック
    if hotkey_str.len() > 256 {
        log::warn!("quick_terminal: hotkey string too long (>{} chars), ignoring", 256);
        return None;
    }

    let parts: Vec<&str> = hotkey_str.split('+').collect();
    if parts.is_empty() {
        return None;
    }

    // DoS 防止: パーツ数の上限チェック
    if parts.len() > 8 {
        log::warn!("quick_terminal: hotkey has too many parts (>8), ignoring");
        return None;
    }

    // 最後の要素がキー名、それ以前が修飾キー
    let (mods_parts, key_part) = parts.split_at(parts.len() - 1);

    let key_str = key_part.first()?;

    // 空のキー名は無効（例: "ctrl+" のような末尾 `+` のケース）
    if key_str.is_empty() {
        log::warn!("quick_terminal: hotkey key name is empty, ignoring");
        return None;
    }

    let mut modifiers = Modifiers::empty();
    for &m in mods_parts {
        match m.to_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            "shift" => modifiers |= Modifiers::SHIFT,
            "alt" | "option" => modifiers |= Modifiers::ALT,
            "super" | "cmd" | "command" | "meta" => modifiers |= Modifiers::SUPER,
            other => {
                log::warn!("quick_terminal: unknown modifier '{}', ignoring", other);
            }
        }
    }

    // キー文字列 → Code マッピング
    let code = match key_str.to_lowercase().as_str() {
        "`" | "backquote" | "backtick" => Code::Backquote,
        "a" => Code::KeyA,
        "b" => Code::KeyB,
        "c" => Code::KeyC,
        "d" => Code::KeyD,
        "e" => Code::KeyE,
        "f" => Code::KeyF,
        "g" => Code::KeyG,
        "h" => Code::KeyH,
        "i" => Code::KeyI,
        "j" => Code::KeyJ,
        "k" => Code::KeyK,
        "l" => Code::KeyL,
        "m" => Code::KeyM,
        "n" => Code::KeyN,
        "o" => Code::KeyO,
        "p" => Code::KeyP,
        "q" => Code::KeyQ,
        "r" => Code::KeyR,
        "s" => Code::KeyS,
        "t" => Code::KeyT,
        "u" => Code::KeyU,
        "v" => Code::KeyV,
        "w" => Code::KeyW,
        "x" => Code::KeyX,
        "y" => Code::KeyY,
        "z" => Code::KeyZ,
        "0" => Code::Digit0,
        "1" => Code::Digit1,
        "2" => Code::Digit2,
        "3" => Code::Digit3,
        "4" => Code::Digit4,
        "5" => Code::Digit5,
        "6" => Code::Digit6,
        "7" => Code::Digit7,
        "8" => Code::Digit8,
        "9" => Code::Digit9,
        "f1" => Code::F1,
        "f2" => Code::F2,
        "f3" => Code::F3,
        "f4" => Code::F4,
        "f5" => Code::F5,
        "f6" => Code::F6,
        "f7" => Code::F7,
        "f8" => Code::F8,
        "f9" => Code::F9,
        "f10" => Code::F10,
        "f11" => Code::F11,
        "f12" => Code::F12,
        "space" => Code::Space,
        "enter" | "return" => Code::Enter,
        "tab" => Code::Tab,
        "escape" | "esc" => Code::Escape,
        "minus" | "-" => Code::Minus,
        "equal" | "=" => Code::Equal,
        "comma" | "," => Code::Comma,
        "period" | "." => Code::Period,
        "slash" | "/" => Code::Slash,
        "backslash" | "\\" => Code::Backslash,
        "semicolon" | ";" => Code::Semicolon,
        "quote" | "'" => Code::Quote,
        "bracketleft" | "[" => Code::BracketLeft,
        "bracketright" | "]" => Code::BracketRight,
        unknown => {
            log::warn!("quick_terminal: unknown key '{}', cannot register hotkey", unknown);
            return None;
        }
    };

    let mods_opt = if modifiers.is_empty() { None } else { Some(modifiers) };
    Some(HotKey::new(mods_opt, code))
}

/// グローバルホットキーを登録する。
///
/// 成功した場合は `(GlobalHotKeyManager, hotkey_id)` を返す。
/// 失敗した場合（アクセシビリティ権限がない等）は `None` を返してログに警告を出す。
#[cfg(target_os = "macos")]
pub(crate) fn register_global_hotkey(
    hotkey_str: &str,
) -> Option<(global_hotkey::GlobalHotKeyManager, u32)> {
    use global_hotkey::GlobalHotKeyManager;

    let hotkey = parse_hotkey(hotkey_str)?;
    let hotkey_id = hotkey.id();

    let manager = match GlobalHotKeyManager::new() {
        Ok(m) => m,
        Err(e) => {
            log::warn!(
                "quick_terminal: failed to create GlobalHotKeyManager: {e}. \
                 Accessibility permission may be required."
            );
            return None;
        }
    };

    match manager.register(hotkey) {
        Ok(()) => {
            log::info!(
                "quick_terminal: registered global hotkey '{}' (id={})",
                hotkey_str,
                hotkey_id
            );
            Some((manager, hotkey_id))
        }
        Err(e) => {
            log::warn!(
                "quick_terminal: failed to register hotkey '{}': {e}. \
                 Accessibility permission may be required.",
                hotkey_str
            );
            None
        }
    }
}

// ---------------------------------------------------------------------------
// ウィンドウ位置計算
// ---------------------------------------------------------------------------

/// Quick Terminal ウィンドウの物理ピクセル座標とサイズを計算する。
///
/// `position` と `size_ratio` に基づいてウィンドウを画面端に配置する。
/// `screen_size` は画面の物理ピクセルサイズ。
/// `t` は 0.0〜1.0 の表示率（0.0 = 完全に画面外、1.0 = 完全に表示）。
pub(crate) fn calc_quick_terminal_geometry(
    position: sdit_core::config::QuickTerminalPosition,
    size_ratio: f32,
    screen_size: (u32, u32),
    t: f32,
) -> (i32, i32, u32, u32) {
    use sdit_core::config::QuickTerminalPosition;

    // screen_size が (0, 0) の場合はフォールバック値を使用する
    let screen_size = if screen_size.0 == 0 || screen_size.1 == 0 {
        log::warn!(
            "calc_quick_terminal_geometry: screen_size is zero, using fallback ({FALLBACK_SCREEN_WIDTH}, {FALLBACK_SCREEN_HEIGHT})"
        );
        (FALLBACK_SCREEN_WIDTH, FALLBACK_SCREEN_HEIGHT)
    } else {
        screen_size
    };

    // size_ratio を安全な範囲に二重 clamp する（MIN_SIZE〜MAX_SIZE）
    // NaN などの異常値も MIN_SIZE〜MAX_SIZE の範囲に収める
    let size_ratio = if size_ratio.is_finite() {
        size_ratio.clamp(
            sdit_core::config::QuickTerminalConfig::MIN_SIZE,
            sdit_core::config::QuickTerminalConfig::MAX_SIZE,
        )
    } else {
        sdit_core::config::QuickTerminalConfig::DEFAULT_SIZE
    };

    // u32 → i32 キャスト: 通常のディスプレイサイズ（最大 ~32767px）では安全
    // i32::MAX は 2,147,483,647 であり、現実的なディスプレイサイズでは溢れない
    let (sw, sh) = (screen_size.0 as i32, screen_size.1 as i32);
    let t = t.clamp(0.0, 1.0);

    match position {
        QuickTerminalPosition::Top => {
            // win_w は画面幅以下に制限
            let win_w = (sw as u32).min(screen_size.0);
            // win_h は画面高さ以下に制限
            let win_h = ((sh as f32 * size_ratio) as u32).max(1).min(screen_size.1);
            // t=0 → y = -(win_h as i32)（画面外上）、t=1 → y = 0
            let y = -(win_h as i32) + ((win_h as f32 * t) as i32);
            (0, y, win_w, win_h)
        }
        QuickTerminalPosition::Bottom => {
            let win_w = (sw as u32).min(screen_size.0);
            let win_h = ((sh as f32 * size_ratio) as u32).max(1).min(screen_size.1);
            // t=0 → y = sh（画面外下）、t=1 → y = sh - win_h
            let y = sh - ((win_h as f32 * t) as i32);
            (0, y, win_w, win_h)
        }
        QuickTerminalPosition::Left => {
            // win_w は画面幅以下に制限
            let win_w = ((sw as f32 * size_ratio) as u32).max(1).min(screen_size.0);
            let win_h = (sh as u32).min(screen_size.1);
            // t=0 → x = -(win_w as i32)（画面外左）、t=1 → x = 0
            let x = -(win_w as i32) + ((win_w as f32 * t) as i32);
            (x, 0, win_w, win_h)
        }
        QuickTerminalPosition::Right => {
            let win_w = ((sw as f32 * size_ratio) as u32).max(1).min(screen_size.0);
            let win_h = (sh as u32).min(screen_size.1);
            // t=0 → x = sw（画面外右）、t=1 → x = sw - win_w
            let x = sw - ((win_w as f32 * t) as i32);
            (x, 0, win_w, win_h)
        }
    }
}

// ---------------------------------------------------------------------------
// テスト
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// FHD 解像度 (1920×1080)。テスト用ローカル定数。
    const SCREEN_FHD: (u32, u32) = (1920, 1080);
    /// QHD 解像度 (2560×1440)。テスト用ローカル定数。
    const SCREEN_QHD: (u32, u32) = (2560, 1440);

    #[test]
    fn animation_ease_in_out_boundaries() {
        assert!((QuickTerminalAnimation::ease_in_out(0.0) - 0.0).abs() < f32::EPSILON);
        assert!((QuickTerminalAnimation::ease_in_out(1.0) - 1.0).abs() < f32::EPSILON);
        // 0.5 では 0.5（対称）
        let mid = QuickTerminalAnimation::ease_in_out(0.5);
        assert!((mid - 0.5).abs() < 1e-5, "ease_in_out(0.5) should be 0.5, got {mid}");
    }

    #[test]
    fn animation_ease_in_out_clamped() {
        // 範囲外入力はクランプされる
        assert!((QuickTerminalAnimation::ease_in_out(-1.0) - 0.0).abs() < f32::EPSILON);
        assert!((QuickTerminalAnimation::ease_in_out(2.0) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn animation_instant_complete_when_zero_duration() {
        let anim = QuickTerminalAnimation {
            start_time: Instant::now(),
            duration_secs: 0.0,
            direction: AnimationDirection::SlideIn,
        };
        assert!(anim.is_complete());
        assert!((anim.progress() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn quick_terminal_state_initial() {
        let state = QuickTerminalState::new();
        assert!(!state.visible);
        assert!(state.animation.is_none());
        assert!(state.window_id.is_none());
        assert!(!state.window_created);
    }

    #[test]
    fn quick_terminal_state_slide_in() {
        let mut state = QuickTerminalState::new();
        state.start_slide_in(0.2);
        assert!(state.visible);
        assert!(state.animation.is_some());
        assert!(!state.is_animating() || state.animation.is_some());
    }

    #[test]
    fn quick_terminal_state_slide_out() {
        let mut state = QuickTerminalState::new();
        state.visible = true;
        state.start_slide_out(0.2);
        // スライドアウト開始後はまだ visible = true（完了してから false になる）
        assert!(state.visible);
        assert!(state.animation.is_some());
    }

    #[test]
    fn quick_terminal_state_finish_slide_out() {
        let mut state = QuickTerminalState::new();
        state.visible = true;
        // start_time を過去に設定してアニメーション完了済み状態を作る（sleep 不要）
        state.animation = Some(QuickTerminalAnimation {
            start_time: Instant::now() - std::time::Duration::from_millis(100),
            duration_secs: 0.001,
            direction: AnimationDirection::SlideOut,
        });
        state.finish_animation();
        assert!(!state.visible);
    }

    #[test]
    fn calc_geometry_top() {
        // t=1.0（完全表示）では y=0 で画面幅のウィンドウ
        let ratio = 0.4f32;
        let (x, y, w, h) = calc_quick_terminal_geometry(
            sdit_core::config::QuickTerminalPosition::Top,
            ratio,
            SCREEN_FHD,
            1.0,
        );
        let expected_h = (SCREEN_FHD.1 as f32 * ratio) as u32;
        assert_eq!(x, 0);
        assert_eq!(y, 0);
        assert_eq!(w, SCREEN_FHD.0);
        assert_eq!(h, expected_h);
    }

    #[test]
    fn calc_geometry_top_hidden() {
        // t=0.0（完全非表示）では y は -win_h
        let (x, y, _w, h) = calc_quick_terminal_geometry(
            sdit_core::config::QuickTerminalPosition::Top,
            0.4,
            SCREEN_FHD,
            0.0,
        );
        assert_eq!(x, 0);
        assert_eq!(y, -(h as i32));
    }

    #[test]
    fn calc_geometry_bottom() {
        let (x, y, w, h) = calc_quick_terminal_geometry(
            sdit_core::config::QuickTerminalPosition::Bottom,
            0.4,
            SCREEN_FHD,
            1.0,
        );
        assert_eq!(x, 0);
        assert_eq!(y, SCREEN_FHD.1 as i32 - h as i32);
        assert_eq!(w, SCREEN_FHD.0);
    }

    #[test]
    fn calc_geometry_left() {
        let (x, y, w, h) = calc_quick_terminal_geometry(
            sdit_core::config::QuickTerminalPosition::Left,
            0.4,
            SCREEN_FHD,
            1.0,
        );
        assert_eq!(x, 0);
        assert_eq!(y, 0);
        assert_eq!(h, SCREEN_FHD.1);
        let _ = w; // w は 1920 * 0.4 = 768
    }

    #[test]
    fn calc_geometry_right() {
        let (x, y, w, h) = calc_quick_terminal_geometry(
            sdit_core::config::QuickTerminalPosition::Right,
            0.4,
            SCREEN_FHD,
            1.0,
        );
        assert_eq!(y, 0);
        assert_eq!(h, SCREEN_FHD.1);
        assert_eq!(x, SCREEN_FHD.0 as i32 - w as i32);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn parse_hotkey_ctrl_backtick() {
        let hk = parse_hotkey("ctrl+`");
        assert!(hk.is_some(), "ctrl+` should be parseable");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn parse_hotkey_ctrl_shift_t() {
        let hk = parse_hotkey("ctrl+shift+t");
        assert!(hk.is_some(), "ctrl+shift+t should be parseable");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn parse_hotkey_empty() {
        let hk = parse_hotkey("");
        assert!(hk.is_none(), "empty string should return None");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn parse_hotkey_unknown_key() {
        let hk = parse_hotkey("ctrl+xyz_unknown");
        assert!(hk.is_none(), "unknown key should return None");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn parse_hotkey_too_long() {
        // 256文字超の文字列は None を返す（DoS 防止）
        let long_str = "a".repeat(257);
        let hk = parse_hotkey(&long_str);
        assert!(hk.is_none(), "string > 256 chars should return None");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn parse_hotkey_trailing_plus() {
        // 末尾が `+` で終わる（空のキー名）は None を返す
        let hk = parse_hotkey("ctrl+");
        assert!(hk.is_none(), "trailing '+' (empty key name) should return None");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn parse_hotkey_too_many_parts() {
        // 9パーツ以上は None を返す（DoS 防止）
        let hk = parse_hotkey("ctrl+shift+alt+super+ctrl+shift+alt+super+t");
        assert!(hk.is_none(), "more than 8 parts should return None");
    }

    #[test]
    fn calc_geometry_zero_screen_size() {
        // screen_size が (0, 0) の場合はフォールバック (FALLBACK_SCREEN_WIDTH, FALLBACK_SCREEN_HEIGHT) を使用する
        let ratio = 0.4f32;
        let (x, y, w, h) = calc_quick_terminal_geometry(
            sdit_core::config::QuickTerminalPosition::Top,
            ratio,
            (0, 0),
            1.0,
        );
        let expected_h = (FALLBACK_SCREEN_HEIGHT as f32 * ratio) as u32;
        // フォールバック値で計算される
        assert_eq!(x, 0);
        assert_eq!(y, 0); // t=1.0 なので y=0
        assert_eq!(w, FALLBACK_SCREEN_WIDTH);
        assert_eq!(h, expected_h);
    }

    #[test]
    fn calc_geometry_four_directions_normal() {
        // 4方向すべてで通常のディスプレイサイズが正しく計算される
        let screen = SCREEN_QHD;
        let ratio = 0.5f32;
        let expected_h_top = (screen.1 as f32 * ratio) as u32; // 縦方向: 1440 * 0.5
        let expected_w_left = (screen.0 as f32 * ratio) as u32; // 横方向: 2560 * 0.5

        // Top: t=1.0
        let (x, _y, w, h) = calc_quick_terminal_geometry(
            sdit_core::config::QuickTerminalPosition::Top,
            ratio,
            screen,
            1.0,
        );
        assert_eq!(x, 0, "Top: x should be 0");
        assert_eq!(w, SCREEN_QHD.0, "Top: width should span full screen");
        assert_eq!(h, expected_h_top, "Top: height should be screen * ratio");

        // Bottom: t=1.0
        let (_x, y, w, h) = calc_quick_terminal_geometry(
            sdit_core::config::QuickTerminalPosition::Bottom,
            ratio,
            screen,
            1.0,
        );
        assert_eq!(w, SCREEN_QHD.0, "Bottom: width should span full screen");
        assert_eq!(y, SCREEN_QHD.1 as i32 - h as i32, "Bottom: y should align to screen bottom");

        // Left: t=1.0
        let (x, y, w, h) = calc_quick_terminal_geometry(
            sdit_core::config::QuickTerminalPosition::Left,
            ratio,
            screen,
            1.0,
        );
        assert_eq!(x, 0, "Left: x should be 0");
        assert_eq!(y, 0, "Left: y should be 0");
        assert_eq!(w, expected_w_left, "Left: width should be screen * ratio");
        assert_eq!(h, SCREEN_QHD.1, "Left: height should span full screen");

        // Right: t=1.0
        let (x, _y, w, h) = calc_quick_terminal_geometry(
            sdit_core::config::QuickTerminalPosition::Right,
            ratio,
            screen,
            1.0,
        );
        assert_eq!(x, SCREEN_QHD.0 as i32 - w as i32, "Right: x should align to screen right");
    }
}
