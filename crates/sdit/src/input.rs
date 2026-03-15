use winit::keyboard::{Key, ModifiersState, NamedKey};

use sdit_core::config::keybinds::{
    Action, KeybindConfig, MOD_BIT_ALT, MOD_BIT_CTRL, MOD_BIT_SHIFT, MOD_BIT_SUPER,
};
use sdit_core::terminal::{KittyKeyboardFlags, TermMode};

// ---------------------------------------------------------------------------
// キーバインド解決
// ---------------------------------------------------------------------------

/// バインディング定義の key 文字列が winit の `Key` にマッチするかどうかを返す。
///
/// - 1 文字の場合: `Key::Character` と大文字小文字無視で比較
/// - `backslash` などのエイリアスは対応する文字に変換
/// - `Tab`, `Enter`, `Backspace`, `Escape` などの名前付きキー: `Key::Named` にマッチ
/// - `[`, `]`, `{`, `}` などの記号: `Key::Character` でマッチ（Shift 状態変化後の文字も許可）
pub(crate) fn key_matches(binding_key: &str, input_key: &Key) -> bool {
    match input_key {
        Key::Character(s) => {
            let input = s.as_str();
            // バインディング定義の key を正規化
            let normalized = normalize_key_alias(binding_key);
            // 大文字小文字無視で比較
            input.eq_ignore_ascii_case(normalized.as_str())
                // Shift により文字が変化するケース: "]" と "}" は同じキー
                || shifted_equivalent(normalized.as_str())
                    .is_some_and(|alt| input.eq_ignore_ascii_case(alt))
        }
        Key::Named(named) => match binding_key.to_ascii_lowercase().as_str() {
            "tab" => *named == NamedKey::Tab,
            "enter" | "return" => *named == NamedKey::Enter,
            "backspace" => *named == NamedKey::Backspace,
            "escape" | "esc" => *named == NamedKey::Escape,
            "space" => *named == NamedKey::Space,
            "pageup" | "page_up" => *named == NamedKey::PageUp,
            "pagedown" | "page_down" => *named == NamedKey::PageDown,
            "home" => *named == NamedKey::Home,
            "end" => *named == NamedKey::End,
            "insert" => *named == NamedKey::Insert,
            "delete" => *named == NamedKey::Delete,
            "arrowup" | "up" => *named == NamedKey::ArrowUp,
            "arrowdown" | "down" => *named == NamedKey::ArrowDown,
            "arrowleft" | "left" => *named == NamedKey::ArrowLeft,
            "arrowright" | "right" => *named == NamedKey::ArrowRight,
            _ => false,
        },
        _ => false,
    }
}

/// 特殊文字のエイリアスを正規化する。
fn normalize_key_alias(key: &str) -> String {
    match key.to_ascii_lowercase().as_str() {
        "backslash" => "\\".to_owned(),
        "plus" => "+".to_owned(),
        other => other.to_owned(),
    }
}

/// Shift キーで変化する文字の代替形（例: `[` → `{`, `]` → `}`）を返す。
fn shifted_equivalent(key: &str) -> Option<&'static str> {
    match key {
        "[" => Some("{"),
        "]" => Some("}"),
        "{" => Some("["),
        "}" => Some("]"),
        _ => None,
    }
}

/// winit の `ModifiersState` をビットフィールドに変換する。
fn mods_to_bits(mods: ModifiersState) -> u8 {
    let mut bits = 0u8;
    if mods.super_key() {
        bits |= MOD_BIT_SUPER;
    }
    if mods.control_key() {
        bits |= MOD_BIT_CTRL;
    }
    if mods.shift_key() {
        bits |= MOD_BIT_SHIFT;
    }
    if mods.alt_key() {
        bits |= MOD_BIT_ALT;
    }
    bits
}

/// キーとモディファイアに対応する `Action` を設定から解決する。
///
/// モディファイアは完全一致で比較する（キャッシュ済みビットフィールド使用）。
/// Character キーは大文字小文字無視で比較する。
/// winit では Shift+"=" が Character("+") + SUPER|SHIFT として届くため、
/// デフォルトバインディングで "plus" は `super|shift` として定義する。
/// アクションと、`unconsumed` フラグ（true = キーを PTY にも転送する）を返す。
pub(crate) fn resolve_action(
    key: &Key,
    mods: ModifiersState,
    config: &KeybindConfig,
) -> Option<(Action, bool)> {
    let input_bits = mods_to_bits(mods);
    for binding in &config.bindings {
        if binding.cached_mods_bits != input_bits {
            continue;
        }
        if key_matches(&binding.key, key) {
            return Some((binding.action, binding.unconsumed));
        }
    }
    None
}

// ---------------------------------------------------------------------------
// URL モディファイア（Action ではない状態チェック）
// ---------------------------------------------------------------------------

/// URL を開くモディファイアキーが押されているかどうか。
///
/// macOS: Cmd、それ以外: Ctrl
pub(crate) fn is_url_modifier(modifiers: ModifiersState) -> bool {
    if cfg!(target_os = "macos") { modifiers.super_key() } else { modifiers.control_key() }
}

// ---------------------------------------------------------------------------
// マウスイベント → PTY バイト列変換
// ---------------------------------------------------------------------------

/// マウスイベントをSGR形式（`CSI < Cb ; Cx ; Cy M/m`）のバイト列に変換する。
///
/// `button`: 0=左, 1=中, 2=右, 32+=ドラッグ修飾, `64`=`scroll_up`, `65`=`scroll_down`
/// `col`, `row`: 0-indexed グリッド座標
/// `pressed`: true=press(M), false=release(m)
pub(crate) fn mouse_report_sgr(button: u8, col: usize, row: usize, pressed: bool) -> Vec<u8> {
    let suffix = if pressed { 'M' } else { 'm' };
    format!("\x1b[<{button};{};{}{suffix}", col.saturating_add(1), row.saturating_add(1))
        .into_bytes()
}

/// マウスイベントをX11形式（`CSI M Cb Cx Cy`）のバイト列に変換する。
///
/// X11形式は座標が 0〜222 の範囲（+33 エンコード）。範囲外は 255 にクランプ。
/// `col`, `row`: 0-indexed グリッド座標
pub(crate) fn mouse_report_x11(button: u8, col: usize, row: usize) -> Vec<u8> {
    let cb = button.saturating_add(32);
    let cx = (col.min(222) as u8).saturating_add(33);
    let cy = (row.min(222) as u8).saturating_add(33);
    vec![b'\x1b', b'[', b'M', cb, cx, cy]
}

/// ピクセル座標をグリッド座標 (col, row) に変換する。
///
/// `sidebar_width_px`: サイドバーの幅（ピクセル）
/// `padding_x`: 左端パディング（ピクセル）
/// `padding_y`: 上端パディング（ピクセル）
pub(crate) fn pixel_to_grid(
    x: f64,
    y: f64,
    cell_width: f32,
    cell_height: f32,
    sidebar_width_px: f32,
    padding_x: f32,
    padding_y: f32,
) -> (usize, usize) {
    let term_x = (x as f32 - sidebar_width_px - padding_x).max(0.0);
    let term_y = (y as f32 - padding_y).max(0.0);
    let col = if cell_width > 0.0 { (term_x / cell_width).floor() as usize } else { 0 };
    let row = if cell_height > 0.0 { (term_y / cell_height).floor() as usize } else { 0 };
    (col, row)
}

// ---------------------------------------------------------------------------
// キー入力 → PTY バイト列変換
// ---------------------------------------------------------------------------

/// Kitty keyboard protocol における修飾キーのエンコーディング。
///
/// Kitty 仕様では修飾子は 1-based（1=なし、2=Shift、3=Alt、...）。
/// ビットフィールド: Shift=1, Alt=2, Ctrl=4, Super=8
fn kitty_modifiers(mods: ModifiersState) -> u8 {
    let mut m = 0u8;
    if mods.shift_key() {
        m |= 1;
    }
    if mods.alt_key() {
        m |= 2;
    }
    if mods.control_key() {
        m |= 4;
    }
    if mods.super_key() {
        m |= 8;
    }
    m
}

/// Kitty CSI u シーケンスを生成する。
///
/// `code`: Unicode コードポイントまたは仮想キーコード
/// `mods`: Kitty 修飾子ビットフィールド（0=なし）
/// `suffix`: 通常は `u`、方向キーなどは `A`/`B`/`C`/`D`
fn kitty_csi(code: u32, mods: u8, suffix: char) -> Vec<u8> {
    if mods == 0 {
        // 修飾なし: CSI code suffix（パラメータ省略）
        format!("\x1b[{code}{suffix}").into_bytes()
    } else {
        // 修飾あり: CSI code ; (mods+1) suffix
        format!("\x1b[{code};{}{suffix}", mods + 1).into_bytes()
    }
}

pub(crate) fn key_to_bytes(
    key: &Key,
    modifiers: ModifiersState,
    mode: TermMode,
    kitty_flags: KittyKeyboardFlags,
) -> Option<Vec<u8>> {
    // Kitty disambiguate モードが有効な場合は CSI u エンコーディングを使用
    if kitty_flags.has(KittyKeyboardFlags::DISAMBIGUATE) {
        return key_to_bytes_kitty(key, modifiers, mode);
    }
    key_to_bytes_legacy(key, modifiers, mode)
}

/// レガシーエンコーディング（Kitty 無効時）。
fn key_to_bytes_legacy(key: &Key, modifiers: ModifiersState, mode: TermMode) -> Option<Vec<u8>> {
    match key {
        Key::Character(s) => {
            let bytes = s.as_bytes();
            if bytes.is_empty() {
                return None;
            }
            if modifiers.control_key() && bytes.len() == 1 {
                let b = bytes[0];
                let ctrl_byte = if b.is_ascii_lowercase() {
                    b - b'a' + 1
                } else if b.is_ascii_uppercase() {
                    b - b'A' + 1
                } else {
                    b
                };
                return Some(vec![ctrl_byte]);
            }
            // Alt → ESC prefix
            if modifiers.alt_key() {
                let mut result = vec![0x1b]; // ESC
                result.extend_from_slice(bytes);
                return Some(result);
            }
            Some(bytes.to_vec())
        }
        Key::Named(named) => {
            let app_cursor = mode.contains(TermMode::APP_CURSOR);
            let s: &[u8] = match named {
                NamedKey::Enter => b"\r",
                NamedKey::Backspace => b"\x7f",
                NamedKey::Tab => b"\t",
                NamedKey::Escape => b"\x1b",
                NamedKey::ArrowUp => {
                    if app_cursor {
                        b"\x1bOA"
                    } else {
                        b"\x1b[A"
                    }
                }
                NamedKey::ArrowDown => {
                    if app_cursor {
                        b"\x1bOB"
                    } else {
                        b"\x1b[B"
                    }
                }
                NamedKey::ArrowRight => {
                    if app_cursor {
                        b"\x1bOC"
                    } else {
                        b"\x1b[C"
                    }
                }
                NamedKey::ArrowLeft => {
                    if app_cursor {
                        b"\x1bOD"
                    } else {
                        b"\x1b[D"
                    }
                }
                NamedKey::Home => b"\x1b[H",
                NamedKey::End => b"\x1b[F",
                NamedKey::PageUp => b"\x1b[5~",
                NamedKey::PageDown => b"\x1b[6~",
                NamedKey::Insert => b"\x1b[2~",
                NamedKey::Delete => b"\x1b[3~",
                NamedKey::Space => b" ",
                _ => return None,
            };
            Some(s.to_vec())
        }
        _ => None,
    }
}

/// Kitty keyboard protocol エンコーディング（`disambiguate` フラグ有効時）。
///
/// 修飾なしの通常キー（Enter, Tab, Backspace 等）はレガシー互換シーケンスを返す。
/// 修飾子がある場合または特殊キーは CSI u 形式を使用する。
fn key_to_bytes_kitty(key: &Key, modifiers: ModifiersState, _mode: TermMode) -> Option<Vec<u8>> {
    let km = kitty_modifiers(modifiers);

    match key {
        Key::Character(s) => {
            let bytes = s.as_bytes();
            if bytes.is_empty() {
                return None;
            }
            // 単一 ASCII 文字の場合のみ Kitty エンコーディングを適用
            if bytes.len() == 1 && bytes[0].is_ascii() {
                let ch = bytes[0] as u32;

                // Ctrl が押されている場合: CSI u を使用
                if modifiers.control_key() {
                    // Ctrl+A〜Z のみ CSI u で送る
                    let base_char = if bytes[0].is_ascii_lowercase() {
                        bytes[0]
                    } else if bytes[0].is_ascii_uppercase() {
                        bytes[0].to_ascii_lowercase()
                    } else {
                        bytes[0]
                    };
                    if base_char.is_ascii_alphabetic() {
                        return Some(kitty_csi(base_char.to_ascii_lowercase() as u32, km, 'u'));
                    }
                    // その他の制御文字はレガシー
                    let ctrl_byte = if bytes[0].is_ascii_lowercase() {
                        bytes[0] - b'a' + 1
                    } else if bytes[0].is_ascii_uppercase() {
                        bytes[0] - b'A' + 1
                    } else {
                        bytes[0]
                    };
                    return Some(vec![ctrl_byte]);
                }

                // 修飾子なしの場合はレガシー（文字そのまま）
                if km == 0 {
                    // Alt → ESC prefix はレガシーと同じ
                    return Some(bytes.to_vec());
                }

                // Alt + 文字 は ESC prefix（レガシー互換）
                if modifiers.alt_key() && !modifiers.control_key() && !modifiers.shift_key() {
                    let mut result = vec![0x1b];
                    result.extend_from_slice(bytes);
                    return Some(result);
                }

                // Shift + 文字: CSI u で Unicode コードポイントを送る
                return Some(kitty_csi(ch, km, 'u'));
            }
            // マルチバイト文字は修飾なしならそのまま
            if km == 0 {
                return Some(bytes.to_vec());
            }
            None
        }
        Key::Named(named) => {
            // 修飾なしの特殊キーはレガシー互換を維持
            match named {
                NamedKey::Enter => {
                    if km == 0 {
                        return Some(b"\r".to_vec());
                    }
                    return Some(kitty_csi(13, km, 'u'));
                }
                NamedKey::Tab => {
                    if km == 0 {
                        return Some(b"\t".to_vec());
                    }
                    return Some(kitty_csi(9, km, 'u'));
                }
                NamedKey::Backspace => {
                    if km == 0 {
                        return Some(b"\x7f".to_vec());
                    }
                    return Some(kitty_csi(127, km, 'u'));
                }
                NamedKey::Escape => {
                    if km == 0 {
                        return Some(b"\x1b".to_vec());
                    }
                    return Some(kitty_csi(27, km, 'u'));
                }
                // 方向キー: CSI 1 ; mods {A,B,C,D}
                NamedKey::ArrowUp => {
                    return Some(kitty_csi(1, km, 'A'));
                }
                NamedKey::ArrowDown => {
                    return Some(kitty_csi(1, km, 'B'));
                }
                NamedKey::ArrowRight => {
                    return Some(kitty_csi(1, km, 'C'));
                }
                NamedKey::ArrowLeft => {
                    return Some(kitty_csi(1, km, 'D'));
                }
                // ナビゲーションキー: CSI code ; mods ~
                NamedKey::Insert => {
                    return Some(kitty_csi(2, km, '~'));
                }
                NamedKey::Delete => {
                    return Some(kitty_csi(3, km, '~'));
                }
                NamedKey::Home => {
                    if km == 0 {
                        return Some(b"\x1b[H".to_vec());
                    }
                    return Some(kitty_csi(1, km, 'H'));
                }
                NamedKey::End => {
                    if km == 0 {
                        return Some(b"\x1b[F".to_vec());
                    }
                    return Some(kitty_csi(1, km, 'F'));
                }
                NamedKey::PageUp => {
                    return Some(kitty_csi(5, km, '~'));
                }
                NamedKey::PageDown => {
                    return Some(kitty_csi(6, km, '~'));
                }
                NamedKey::Space => {
                    if km == 0 {
                        return Some(b" ".to_vec());
                    }
                    return Some(kitty_csi(32, km, 'u'));
                }
                // ファンクションキー
                NamedKey::F1 => return Some(kitty_csi(1, km, 'P')),
                NamedKey::F2 => return Some(kitty_csi(1, km, 'Q')),
                NamedKey::F3 => return Some(kitty_csi(1, km, 'R')),
                NamedKey::F4 => return Some(kitty_csi(1, km, 'S')),
                NamedKey::F5 => return Some(kitty_csi(15, km, '~')),
                NamedKey::F6 => return Some(kitty_csi(17, km, '~')),
                NamedKey::F7 => return Some(kitty_csi(18, km, '~')),
                NamedKey::F8 => return Some(kitty_csi(19, km, '~')),
                NamedKey::F9 => return Some(kitty_csi(20, km, '~')),
                NamedKey::F10 => return Some(kitty_csi(21, km, '~')),
                NamedKey::F11 => return Some(kitty_csi(23, km, '~')),
                NamedKey::F12 => return Some(kitty_csi(24, km, '~')),
                _ => return None,
            }
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// テスト
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use winit::keyboard::SmolStr;

    use super::*;

    fn char_key(c: &str) -> Key {
        Key::Character(SmolStr::new(c))
    }

    /// テスト用: mods 文字列を `ModifiersState` にパースする。
    fn parse_mods(s: &str) -> ModifiersState {
        let mut state = ModifiersState::empty();
        for token in s.split('|') {
            match token.trim().to_ascii_lowercase().as_str() {
                "super" | "cmd" | "logo" => state |= ModifiersState::SUPER,
                "ctrl" | "control" => state |= ModifiersState::CONTROL,
                "shift" => state |= ModifiersState::SHIFT,
                "alt" | "option" => state |= ModifiersState::ALT,
                _ => {}
            }
        }
        state
    }

    // --- parse_mods ---

    #[test]
    fn parse_mods_empty() {
        assert_eq!(parse_mods(""), ModifiersState::empty());
    }

    #[test]
    fn parse_mods_super() {
        assert_eq!(parse_mods("super"), ModifiersState::SUPER);
    }

    #[test]
    fn parse_mods_ctrl() {
        assert_eq!(parse_mods("ctrl"), ModifiersState::CONTROL);
    }

    #[test]
    fn parse_mods_super_shift() {
        let expected = ModifiersState::SUPER | ModifiersState::SHIFT;
        assert_eq!(parse_mods("super|shift"), expected);
    }

    #[test]
    fn parse_mods_ctrl_shift() {
        let expected = ModifiersState::CONTROL | ModifiersState::SHIFT;
        assert_eq!(parse_mods("ctrl|shift"), expected);
    }

    #[test]
    fn parse_mods_case_insensitive() {
        assert_eq!(parse_mods("SUPER"), ModifiersState::SUPER);
        assert_eq!(parse_mods("CTRL|SHIFT"), ModifiersState::CONTROL | ModifiersState::SHIFT);
    }

    #[test]
    fn parse_mods_alias_cmd() {
        assert_eq!(parse_mods("cmd"), ModifiersState::SUPER);
    }

    // --- key_matches ---

    #[test]
    fn key_matches_character() {
        assert!(key_matches("n", &char_key("n")));
        assert!(key_matches("n", &char_key("N"))); // 大文字小文字無視
        assert!(!key_matches("n", &char_key("t")));
    }

    #[test]
    fn key_matches_backslash_alias() {
        assert!(key_matches("backslash", &char_key("\\")));
    }

    #[test]
    fn key_matches_plus_alias() {
        assert!(key_matches("plus", &char_key("+")));
    }

    #[test]
    fn key_matches_bracket_shifted_equivalent() {
        // "]" バインディングは "}" (Shift+]) にもマッチ
        assert!(key_matches("]", &char_key("}")));
        assert!(key_matches("[", &char_key("{")));
    }

    #[test]
    fn key_matches_tab() {
        assert!(key_matches("Tab", &Key::Named(NamedKey::Tab)));
        assert!(!key_matches("Tab", &char_key("t")));
    }

    #[test]
    fn key_matches_enter() {
        assert!(key_matches("Enter", &Key::Named(NamedKey::Enter)));
        assert!(key_matches("enter", &Key::Named(NamedKey::Enter)));
    }

    #[test]
    fn key_matches_escape() {
        assert!(key_matches("Escape", &Key::Named(NamedKey::Escape)));
        assert!(key_matches("esc", &Key::Named(NamedKey::Escape)));
    }

    // --- resolve_action ---

    fn make_config(bindings: Vec<(String, String, Action)>) -> KeybindConfig {
        use sdit_core::config::keybinds::KeyBinding;
        let mut config = KeybindConfig {
            bindings: bindings
                .into_iter()
                .map(|(key, mods, action)| KeyBinding {
                    key,
                    mods,
                    action,
                    unconsumed: false,
                    cached_mods_bits: 0,
                })
                .collect(),
        };
        config.validate();
        config
    }

    #[test]
    fn resolve_action_basic() {
        let config = make_config(vec![("n".to_owned(), "super".to_owned(), Action::NewWindow)]);
        let result = resolve_action(&char_key("n"), ModifiersState::SUPER, &config);
        assert_eq!(result, Some((Action::NewWindow, false)));
    }

    #[test]
    fn resolve_action_no_match() {
        let config = make_config(vec![("n".to_owned(), "super".to_owned(), Action::NewWindow)]);
        // モディファイアが違う
        let result = resolve_action(&char_key("n"), ModifiersState::CONTROL, &config);
        assert_eq!(result, None::<(Action, bool)>);
    }

    #[test]
    fn resolve_action_shift_super_n_vs_super_n() {
        // super+n と super+shift+n は区別される
        let config = make_config(vec![
            ("n".to_owned(), "super".to_owned(), Action::NewWindow),
            ("n".to_owned(), "super|shift".to_owned(), Action::DetachSession),
        ]);
        let new_win = resolve_action(&char_key("n"), ModifiersState::SUPER, &config);
        let detach =
            resolve_action(&char_key("N"), ModifiersState::SUPER | ModifiersState::SHIFT, &config);
        assert_eq!(new_win, Some((Action::NewWindow, false)));
        assert_eq!(detach, Some((Action::DetachSession, false)));
    }

    #[test]
    fn resolve_action_zoom_in_with_plus_binding() {
        // "plus" バインディングは "super|shift" で、"+" キー (SUPER|SHIFT) にマッチ
        let config =
            make_config(vec![("plus".to_owned(), "super|shift".to_owned(), Action::ZoomIn)]);
        // "+" は Shift+"=" なので SUPER|SHIFT で来る
        let result =
            resolve_action(&char_key("+"), ModifiersState::SUPER | ModifiersState::SHIFT, &config);
        assert_eq!(result, Some((Action::ZoomIn, false)));
    }

    #[test]
    fn resolve_action_zoom_in_eq_no_shift() {
        // "=" バインディングは SUPER のみでマッチ
        let config = make_config(vec![("=".to_owned(), "super".to_owned(), Action::ZoomIn)]);
        let result = resolve_action(&char_key("="), ModifiersState::SUPER, &config);
        assert_eq!(result, Some((Action::ZoomIn, false)));
        // "+" (SUPER|SHIFT) は "=" バインディングにはマッチしない
        let result_plus =
            resolve_action(&char_key("+"), ModifiersState::SUPER | ModifiersState::SHIFT, &config);
        assert_eq!(result_plus, None);
    }

    #[test]
    fn resolve_action_tab_named_key() {
        let config = make_config(vec![("Tab".to_owned(), "ctrl".to_owned(), Action::NextSession)]);
        let result = resolve_action(&Key::Named(NamedKey::Tab), ModifiersState::CONTROL, &config);
        assert_eq!(result, Some((Action::NextSession, false)));
    }

    #[test]
    fn resolve_action_backslash_alias() {
        let config =
            make_config(vec![("backslash".to_owned(), "super".to_owned(), Action::SidebarToggle)]);
        let result = resolve_action(&char_key("\\"), ModifiersState::SUPER, &config);
        assert_eq!(result, Some((Action::SidebarToggle, false)));
    }

    // --- is_url_modifier ---

    #[cfg(target_os = "macos")]
    #[test]
    fn url_modifier_super_key() {
        let mods = ModifiersState::SUPER;
        assert!(is_url_modifier(mods));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn url_modifier_no_modifier() {
        let mods = ModifiersState::empty();
        assert!(!is_url_modifier(mods));
    }

    // --- デフォルトバインディングの重複チェック ---

    #[cfg(target_os = "macos")]
    #[test]
    fn default_bindings_zoom_actions_exist() {
        use sdit_core::config::keybinds::default_bindings;
        let bindings = default_bindings();
        let has_zoom_in = bindings.iter().any(|b| b.action == Action::ZoomIn);
        let has_zoom_out = bindings.iter().any(|b| b.action == Action::ZoomOut);
        let has_zoom_reset = bindings.iter().any(|b| b.action == Action::ZoomReset);
        assert!(has_zoom_in, "ZoomIn バインディングが存在しない");
        assert!(has_zoom_out, "ZoomOut バインディングが存在しない");
        assert!(has_zoom_reset, "ZoomReset バインディングが存在しない");
    }

    // ---------------------------------------------------------------------------
    // Kitty keyboard encoding テスト
    // ---------------------------------------------------------------------------

    fn no_mods() -> ModifiersState {
        ModifiersState::empty()
    }

    fn kitty_mode() -> KittyKeyboardFlags {
        KittyKeyboardFlags::from_raw(KittyKeyboardFlags::DISAMBIGUATE)
    }

    fn legacy_mode() -> KittyKeyboardFlags {
        KittyKeyboardFlags::NONE
    }

    #[test]
    fn kitty_disabled_uses_legacy_enter() {
        // kitty_flags が NONE なら Enter は \r
        let result = key_to_bytes(
            &Key::Named(NamedKey::Enter),
            no_mods(),
            TermMode::default(),
            legacy_mode(),
        );
        assert_eq!(result, Some(b"\r".to_vec()));
    }

    #[test]
    fn kitty_disabled_uses_legacy_arrow() {
        // Kitty なし: Up は CSI A
        let result = key_to_bytes(
            &Key::Named(NamedKey::ArrowUp),
            no_mods(),
            TermMode::default(),
            legacy_mode(),
        );
        assert_eq!(result, Some(b"\x1b[A".to_vec()));
    }

    #[test]
    fn kitty_enter_no_mods_is_legacy() {
        // Kitty mode でも修飾なし Enter は \r
        let result = key_to_bytes(
            &Key::Named(NamedKey::Enter),
            no_mods(),
            TermMode::default(),
            kitty_mode(),
        );
        assert_eq!(result, Some(b"\r".to_vec()));
    }

    #[test]
    fn kitty_arrow_up_no_mods() {
        // Kitty mode で Up は CSI 1A（修飾なし）
        let result = key_to_bytes(
            &Key::Named(NamedKey::ArrowUp),
            no_mods(),
            TermMode::default(),
            kitty_mode(),
        );
        assert_eq!(result, Some(b"\x1b[1A".to_vec()));
    }

    #[test]
    fn kitty_arrow_up_shift() {
        // Kitty mode で Shift+Up は CSI 1;2A
        let result = key_to_bytes(
            &Key::Named(NamedKey::ArrowUp),
            ModifiersState::SHIFT,
            TermMode::default(),
            kitty_mode(),
        );
        assert_eq!(result, Some(b"\x1b[1;2A".to_vec()));
    }

    #[test]
    fn kitty_enter_with_ctrl() {
        // Kitty mode で Ctrl+Enter は CSI 13;5u
        let result = key_to_bytes(
            &Key::Named(NamedKey::Enter),
            ModifiersState::CONTROL,
            TermMode::default(),
            kitty_mode(),
        );
        // mods=4 (ctrl), m+1=5
        assert_eq!(result, Some(b"\x1b[13;5u".to_vec()));
    }

    #[test]
    fn kitty_backspace_with_alt() {
        // Kitty mode で Alt+Backspace は CSI 127;3u
        let result = key_to_bytes(
            &Key::Named(NamedKey::Backspace),
            ModifiersState::ALT,
            TermMode::default(),
            kitty_mode(),
        );
        // mods=2 (alt), m+1=3
        assert_eq!(result, Some(b"\x1b[127;3u".to_vec()));
    }

    #[test]
    fn kitty_f1_no_mods() {
        // Kitty mode で F1 は CSI 1P
        let result =
            key_to_bytes(&Key::Named(NamedKey::F1), no_mods(), TermMode::default(), kitty_mode());
        assert_eq!(result, Some(b"\x1b[1P".to_vec()));
    }

    #[test]
    fn kitty_f5_no_mods() {
        // Kitty mode で F5 は CSI 15~
        let result =
            key_to_bytes(&Key::Named(NamedKey::F5), no_mods(), TermMode::default(), kitty_mode());
        assert_eq!(result, Some(b"\x1b[15~".to_vec()));
    }

    #[test]
    fn kitty_delete_with_ctrl() {
        // Kitty mode で Ctrl+Delete は CSI 3;5~
        let result = key_to_bytes(
            &Key::Named(NamedKey::Delete),
            ModifiersState::CONTROL,
            TermMode::default(),
            kitty_mode(),
        );
        assert_eq!(result, Some(b"\x1b[3;5~".to_vec()));
    }

    // ---------------------------------------------------------------------------
    // unconsumed フラグテスト
    // ---------------------------------------------------------------------------

    /// unconsumed 対応のコンフィグヘルパー
    fn make_config_with_unconsumed(bindings: Vec<(String, String, Action, bool)>) -> KeybindConfig {
        use sdit_core::config::keybinds::KeyBinding;
        let mut config = KeybindConfig {
            bindings: bindings
                .into_iter()
                .map(|(key, mods, action, unconsumed)| KeyBinding {
                    key,
                    mods,
                    action,
                    unconsumed,
                    cached_mods_bits: 0,
                })
                .collect(),
        };
        config.validate();
        config
    }

    #[test]
    fn resolve_action_unconsumed_true() {
        // unconsumed = true のバインディングは (action, true) を返す
        let config = make_config_with_unconsumed(vec![(
            "k".to_owned(),
            "super".to_owned(),
            Action::Search,
            true,
        )]);
        let result = resolve_action(&char_key("k"), ModifiersState::SUPER, &config);
        assert_eq!(result, Some((Action::Search, true)));
    }

    #[test]
    fn resolve_action_unconsumed_false_default() {
        // unconsumed = false（デフォルト）のバインディングは (action, false) を返す
        let config = make_config_with_unconsumed(vec![(
            "k".to_owned(),
            "super".to_owned(),
            Action::Search,
            false,
        )]);
        let result = resolve_action(&char_key("k"), ModifiersState::SUPER, &config);
        assert_eq!(result, Some((Action::Search, false)));
    }

    #[test]
    fn resolve_action_unconsumed_mixed() {
        // 同じキーで unconsumed が異なるバインディング（モディファイア違い）
        let config = make_config_with_unconsumed(vec![
            ("k".to_owned(), "super".to_owned(), Action::Search, true),
            ("k".to_owned(), "ctrl".to_owned(), Action::Search, false),
        ]);
        let result_super = resolve_action(&char_key("k"), ModifiersState::SUPER, &config);
        let result_ctrl = resolve_action(&char_key("k"), ModifiersState::CONTROL, &config);
        assert_eq!(result_super, Some((Action::Search, true)));
        assert_eq!(result_ctrl, Some((Action::Search, false)));
    }
}
