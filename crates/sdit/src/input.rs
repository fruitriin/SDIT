use winit::keyboard::{Key, ModifiersState, NamedKey};

use sdit_core::config::keybinds::{
    Action, KeybindConfig, MOD_BIT_ALT, MOD_BIT_CTRL, MOD_BIT_SHIFT, MOD_BIT_SUPER,
};
use sdit_core::terminal::TermMode;

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
pub(crate) fn resolve_action(
    key: &Key,
    mods: ModifiersState,
    config: &KeybindConfig,
) -> Option<Action> {
    let input_bits = mods_to_bits(mods);
    for binding in &config.bindings {
        if binding.cached_mods_bits != input_bits {
            continue;
        }
        if key_matches(&binding.key, key) {
            return Some(binding.action);
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
pub(crate) fn pixel_to_grid(
    x: f64,
    y: f64,
    cell_width: f32,
    cell_height: f32,
    sidebar_width_px: f32,
) -> (usize, usize) {
    let term_x = (x as f32 - sidebar_width_px).max(0.0);
    let col = if cell_width > 0.0 { (term_x / cell_width).floor() as usize } else { 0 };
    let row = if cell_height > 0.0 { (y as f32 / cell_height).floor() as usize } else { 0 };
    (col, row)
}

// ---------------------------------------------------------------------------
// キー入力 → PTY バイト列変換
// ---------------------------------------------------------------------------

pub(crate) fn key_to_bytes(
    key: &Key,
    modifiers: ModifiersState,
    mode: TermMode,
) -> Option<Vec<u8>> {
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
                .map(|(key, mods, action)| KeyBinding { key, mods, action, cached_mods_bits: 0 })
                .collect(),
        };
        config.validate();
        config
    }

    #[test]
    fn resolve_action_basic() {
        let config = make_config(vec![("n".to_owned(), "super".to_owned(), Action::NewWindow)]);
        let result = resolve_action(&char_key("n"), ModifiersState::SUPER, &config);
        assert_eq!(result, Some(Action::NewWindow));
    }

    #[test]
    fn resolve_action_no_match() {
        let config = make_config(vec![("n".to_owned(), "super".to_owned(), Action::NewWindow)]);
        // モディファイアが違う
        let result = resolve_action(&char_key("n"), ModifiersState::CONTROL, &config);
        assert_eq!(result, None);
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
        assert_eq!(new_win, Some(Action::NewWindow));
        assert_eq!(detach, Some(Action::DetachSession));
    }

    #[test]
    fn resolve_action_zoom_in_with_plus_binding() {
        // "plus" バインディングは "super|shift" で、"+" キー (SUPER|SHIFT) にマッチ
        let config =
            make_config(vec![("plus".to_owned(), "super|shift".to_owned(), Action::ZoomIn)]);
        // "+" は Shift+"=" なので SUPER|SHIFT で来る
        let result =
            resolve_action(&char_key("+"), ModifiersState::SUPER | ModifiersState::SHIFT, &config);
        assert_eq!(result, Some(Action::ZoomIn));
    }

    #[test]
    fn resolve_action_zoom_in_eq_no_shift() {
        // "=" バインディングは SUPER のみでマッチ
        let config = make_config(vec![("=".to_owned(), "super".to_owned(), Action::ZoomIn)]);
        let result = resolve_action(&char_key("="), ModifiersState::SUPER, &config);
        assert_eq!(result, Some(Action::ZoomIn));
        // "+" (SUPER|SHIFT) は "=" バインディングにはマッチしない
        let result_plus =
            resolve_action(&char_key("+"), ModifiersState::SUPER | ModifiersState::SHIFT, &config);
        assert_eq!(result_plus, None);
    }

    #[test]
    fn resolve_action_tab_named_key() {
        let config = make_config(vec![("Tab".to_owned(), "ctrl".to_owned(), Action::NextSession)]);
        let result = resolve_action(&Key::Named(NamedKey::Tab), ModifiersState::CONTROL, &config);
        assert_eq!(result, Some(Action::NextSession));
    }

    #[test]
    fn resolve_action_backslash_alias() {
        let config =
            make_config(vec![("backslash".to_owned(), "super".to_owned(), Action::SidebarToggle)]);
        let result = resolve_action(&char_key("\\"), ModifiersState::SUPER, &config);
        assert_eq!(result, Some(Action::SidebarToggle));
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
}
