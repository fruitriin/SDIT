use winit::keyboard::{Key, ModifiersState, NamedKey};

use sdit_core::terminal::TermMode;

// ---------------------------------------------------------------------------
// 新規ウィンドウショートカット判定
// ---------------------------------------------------------------------------

/// Cmd+\ (macOS) または Ctrl+\ でのサイドバートグルかどうか。
pub(crate) fn is_sidebar_toggle_shortcut(key: &Key, modifiers: ModifiersState) -> bool {
    let is_backslash = matches!(key, Key::Character(s) if s.as_str() == "\\" || s.as_str() == "|");
    if !is_backslash {
        return false;
    }
    if cfg!(target_os = "macos") && modifiers.super_key() {
        return true;
    }
    modifiers.control_key()
}

/// Cmd+Shift+N (macOS) でのセッション切出しショートカットかどうか。
pub(crate) fn is_detach_session_shortcut(key: &Key, modifiers: ModifiersState) -> bool {
    let is_n = matches!(key, Key::Character(s) if s.as_str() == "n" || s.as_str() == "N");
    if !is_n {
        return false;
    }
    // macOS: Cmd+Shift+N
    if cfg!(target_os = "macos") && modifiers.super_key() && modifiers.shift_key() {
        return true;
    }
    false
}

/// Cmd+N (macOS) または Ctrl+Shift+N でのウィンドウ生成ショートカットかどうか。
pub(crate) fn is_new_window_shortcut(key: &Key, modifiers: ModifiersState) -> bool {
    let is_n = matches!(key, Key::Character(s) if s.as_str() == "n" || s.as_str() == "N");
    if !is_n {
        return false;
    }
    // macOS: Cmd+N
    if cfg!(target_os = "macos") && modifiers.super_key() && !modifiers.shift_key() {
        return true;
    }
    // Other: Ctrl+Shift+N
    if modifiers.control_key() && modifiers.shift_key() {
        return true;
    }
    false
}

/// Cmd+T (macOS) または Ctrl+Shift+T でのセッション追加ショートカットかどうか。
pub(crate) fn is_add_session_shortcut(key: &Key, modifiers: ModifiersState) -> bool {
    let is_t = matches!(key, Key::Character(s) if s.as_str() == "t" || s.as_str() == "T");
    if !is_t {
        return false;
    }
    if cfg!(target_os = "macos") && modifiers.super_key() && !modifiers.shift_key() {
        return true;
    }
    modifiers.control_key() && modifiers.shift_key()
}

/// Cmd+W (macOS) または Ctrl+Shift+W でのセッション閉じショートカットかどうか。
pub(crate) fn is_close_session_shortcut(key: &Key, modifiers: ModifiersState) -> bool {
    let is_w = matches!(key, Key::Character(s) if s.as_str() == "w" || s.as_str() == "W");
    if !is_w {
        return false;
    }
    if cfg!(target_os = "macos") && modifiers.super_key() && !modifiers.shift_key() {
        return true;
    }
    modifiers.control_key() && modifiers.shift_key()
}

/// セッション切替ショートカット。次: +1、前: -1 を返す。
pub(crate) fn session_switch_direction(key: &Key, modifiers: ModifiersState) -> Option<i32> {
    match key {
        // Ctrl+Tab → 次、Ctrl+Shift+Tab → 前
        Key::Named(NamedKey::Tab) if modifiers.control_key() => {
            Some(if modifiers.shift_key() { -1 } else { 1 })
        }
        // Cmd+Shift+] → 次（macOS）
        Key::Character(s) if s.as_str() == "]" || s.as_str() == "}" => {
            if cfg!(target_os = "macos") && modifiers.super_key() && modifiers.shift_key() {
                Some(1)
            } else {
                None
            }
        }
        // Cmd+Shift+[ → 前（macOS）
        Key::Character(s) if s.as_str() == "[" || s.as_str() == "{" => {
            if cfg!(target_os = "macos") && modifiers.super_key() && modifiers.shift_key() {
                Some(-1)
            } else {
                None
            }
        }
        _ => None,
    }
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
