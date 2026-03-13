//! CSI, ESC, and SGR dispatch handlers for [`Terminal`].
//!
//! Functions here are called from the [`vte::Perform`] implementation in
//! `terminal/mod.rs` and mutate the [`Terminal`] state accordingly.

use vte::Params;

use crate::grid::Dimensions;
use crate::index::{Column, Line, Point};

use super::{KittyKeyboardFlags, MAX_PENDING_WRITES, TermMode, Terminal};

// ---------------------------------------------------------------------------
// CSI dispatch
// ---------------------------------------------------------------------------

/// Handle a complete CSI sequence.
#[allow(clippy::too_many_lines)]
pub fn csi_dispatch(term: &mut Terminal, params: &Params, intermediates: &[u8], action: char) {
    // Private sequences have '?' as the first intermediate byte.
    let is_private = intermediates.first() == Some(&b'?');

    match action {
        // ---- Cursor movement ----

        // CUU — cursor up N
        'A' => {
            let n = i32::try_from(first_param(params, 1)).unwrap_or(i32::MAX);
            let min_line = i32::try_from(term.scroll_region.start).unwrap_or(0);
            let new_line = term.grid.cursor.point.line.0.saturating_sub(n).max(min_line);
            term.grid.cursor.point.line = Line(new_line);
            term.grid.cursor.input_needs_wrap = false;
        }
        // CUD — cursor down N
        'B' => {
            let n = i32::try_from(first_param(params, 1)).unwrap_or(i32::MAX);
            let max_line =
                i32::try_from(term.grid.screen_lines()).unwrap_or(i32::MAX).saturating_sub(1);
            let new_line = term.grid.cursor.point.line.0.saturating_add(n).min(max_line);
            term.grid.cursor.point.line = Line(new_line);
            term.grid.cursor.input_needs_wrap = false;
        }
        // CUF — cursor right N
        'C' => {
            let n = first_param(params, 1);
            let max_col = term.grid.columns().saturating_sub(1);
            let new_col = term.grid.cursor.point.column.0.saturating_add(n).min(max_col);
            term.grid.cursor.point.column = Column(new_col);
            term.grid.cursor.input_needs_wrap = false;
        }
        // CUB — cursor left N
        'D' => {
            let n = first_param(params, 1);
            let new_col = term.grid.cursor.point.column.0.saturating_sub(n);
            term.grid.cursor.point.column = Column(new_col);
            term.grid.cursor.input_needs_wrap = false;
        }
        // CUP / HVP — cursor position (row, col) — 1-based
        'H' | 'f' => {
            let row = first_param(params, 1).saturating_sub(1);
            let col = nth_param(params, 1, 1).saturating_sub(1);
            term.set_cursor(row, col);
        }

        // ---- Erase ----

        // ED — erase display
        'J' => {
            let mode = first_param(params, 0);
            let lines = term.grid.screen_lines();
            let cols = term.grid.columns();
            match mode {
                // Erase from cursor to end of screen
                0 => {
                    term.erase_to_eol();
                    let next_line = term.grid.cursor.point.line.as_viewport_idx() + 1;
                    for ln in next_line..lines {
                        let ln_i32 = i32::try_from(ln).unwrap_or(i32::MAX);
                        let start = Point::new(Line(ln_i32), Column(0));
                        let end = Point::new(Line(ln_i32), Column(cols.saturating_sub(1)));
                        term.erase_cells(start, end);
                    }
                }
                // Erase from start of screen to cursor
                1 => {
                    term.erase_to_bol();
                    let cur_line = term.grid.cursor.point.line.as_viewport_idx();
                    for ln in 0..cur_line {
                        let ln_i32 = i32::try_from(ln).unwrap_or(i32::MAX);
                        let start = Point::new(Line(ln_i32), Column(0));
                        let end = Point::new(Line(ln_i32), Column(cols.saturating_sub(1)));
                        term.erase_cells(start, end);
                    }
                }
                // Erase entire display
                2 => {
                    term.grid.clear_viewport();
                }
                // Erase entire display + scrollback
                3 => {
                    term.grid.clear_viewport();
                    term.grid.clear_history();
                }
                _ => {}
            }
        }

        // EL — erase line
        'K' => {
            let mode = first_param(params, 0);
            match mode {
                0 => term.erase_to_eol(),
                1 => term.erase_to_bol(),
                2 => term.erase_line(),
                _ => {}
            }
        }

        // ---- Line insert/delete ----

        // IL — insert N lines at cursor
        'L' => {
            let n = first_param(params, 1);
            let cur_line = term.grid.cursor.point.line.as_viewport_idx();
            let region = cur_line..term.scroll_region.end;
            if cur_line < term.scroll_region.end {
                term.grid.scroll_down(region, n);
            }
        }

        // DL — delete N lines at cursor
        'M' => {
            let n = first_param(params, 1);
            let cur_line = term.grid.cursor.point.line.as_viewport_idx();
            let region = cur_line..term.scroll_region.end;
            if cur_line < term.scroll_region.end {
                term.grid.scroll_up(region, n);
            }
        }

        // ---- Scroll ----

        // SU — scroll up N lines
        'S' => {
            let n = first_param(params, 1);
            term.grid.scroll_up(term.scroll_region.clone(), n);
        }

        // SD — scroll down N lines
        'T' => {
            let n = first_param(params, 1);
            term.grid.scroll_down(term.scroll_region.clone(), n);
        }

        // ---- SGR — select graphic rendition ----
        'm' => {
            dispatch_sgr(term, params);
        }

        // ---- DECSTBM — set scroll region ----
        'r' => {
            let top = first_param(params, 1).saturating_sub(1);
            let bottom = nth_param(params, 1, term.grid.screen_lines());
            if top < bottom && bottom <= term.grid.screen_lines() {
                term.scroll_region = top..bottom;
            }
            // Reset cursor to home.
            term.set_cursor(0, 0);
        }

        // ---- SM / RM — set/reset mode ----
        'h' | 'l' => {
            let set = action == 'h';
            if is_private {
                for param in params {
                    let p = param.first().copied().unwrap_or(0);
                    set_private_mode(term, p, set);
                }
            } else {
                for param in params {
                    let p = param.first().copied().unwrap_or(0);
                    set_ansi_mode(term, p, set);
                }
            }
        }

        // ---- DA1 / DA2 — Device Attributes ----
        'c' => {
            if intermediates.first() == Some(&b'>') {
                // CSI > c — DA2 (Secondary Device Attributes)
                // CSI > 0 ; 0 ; 0 c — VT100 互換、バージョン 0
                write_response(term, b"\x1b[>0;0;0c");
            } else if !is_private {
                let p = first_param(params, 0);
                if p == 0 {
                    // CSI c / CSI 0 c — DA1 (Primary Device Attributes)
                    // CSI ? 62 ; 4 c — VT220 互換、sixel なし
                    write_response(term, b"\x1b[?62;4c");
                }
            }
        }

        // ---- DSR — Device Status Report / CPR — Cursor Position Report ----
        'n' => {
            if !is_private {
                let p = first_param(params, 0);
                match p {
                    // CSI 5 n → CSI 0 n (端末OK)
                    5 => {
                        write_response(term, b"\x1b[0n");
                    }
                    // CSI 6 n → CSI row ; col R (Cursor Position Report)
                    6 => {
                        // 1-based
                        let row = term.grid.cursor.point.line.as_viewport_idx() + 1;
                        let col = term.grid.cursor.point.column.0 + 1;
                        let response = format!("\x1b[{row};{col}R");
                        write_response(term, response.as_bytes());
                    }
                    _ => {}
                }
            }
        }

        // ---- DECSCUSR — Set Cursor Style ----
        'q' => {
            if intermediates.first() == Some(&b' ') {
                let style = first_param(params, 0);
                match style {
                    0..=2 => {
                        term.cursor_style = super::CursorStyle::Block;
                        term.cursor_blinking = style != 2;
                    }
                    3 | 4 => {
                        term.cursor_style = super::CursorStyle::Underline;
                        term.cursor_blinking = style == 3;
                    }
                    5 | 6 => {
                        term.cursor_style = super::CursorStyle::Bar;
                        term.cursor_blinking = style == 5;
                    }
                    _ => {}
                }
            }
        }

        // ---- Kitty Keyboard Protocol ----

        // CSI > flags u — push keyboard flags
        'u' if intermediates.first() == Some(&b'>') => {
            let flags = params.iter().next().and_then(|p| p.first().copied()).unwrap_or(0);
            let flags = KittyKeyboardFlags::from_raw(flags as u8);
            term.kitty_flags.push(flags);
            log::debug!("Kitty keyboard: push flags {}", flags.raw());
        }

        // CSI < n u — pop N keyboard flags
        'u' if intermediates.first() == Some(&b'<') => {
            let n = params.iter().next().and_then(|p| p.first().copied()).unwrap_or(1) as usize;
            term.kitty_flags.pop(n);
            log::debug!("Kitty keyboard: pop {} flags", n);
        }

        // CSI ? u — query current keyboard flags
        'u' if is_private => {
            let current = term.kitty_flags.current().raw();
            let response = format!("\x1b[?{current}u");
            write_response(term, response.as_bytes());
            log::debug!("Kitty keyboard: query → {}", current);
        }

        _ => {}
    }
}

// ---------------------------------------------------------------------------
// ESC dispatch
// ---------------------------------------------------------------------------

/// Handle an escape sequence (non-CSI).
pub fn esc_dispatch(term: &mut Terminal, intermediates: &[u8], byte: u8) {
    // Only plain ESC sequences (no intermediates or '#') are handled here.
    if !intermediates.is_empty() {
        return;
    }
    match byte {
        // DECSC — save cursor
        b'7' => {
            term.grid.saved_cursor = term.grid.cursor.clone();
        }
        // DECRC — restore cursor
        b'8' => {
            term.grid.cursor = term.grid.saved_cursor.clone();
        }
        // IND — line feed (like LF)
        b'D' => {
            term.linefeed();
        }
        // NEL — next line (LF + CR)
        b'E' => {
            term.linefeed();
            term.carriage_return();
        }
        // RI — reverse index
        b'M' => {
            let top = term.scroll_region.start;
            let cur_line = term.grid.cursor.point.line.as_viewport_idx();
            if cur_line == top {
                term.grid.scroll_down(term.scroll_region.clone(), 1);
            } else if cur_line > 0 {
                term.grid.cursor.point.line -= 1;
            }
            term.grid.cursor.input_needs_wrap = false;
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// SGR helper
// ---------------------------------------------------------------------------

/// Process all parameters of an SGR (`m`) sequence.
fn dispatch_sgr(term: &mut Terminal, params: &Params) {
    let mut iter = params.iter();
    while let Some(param) = iter.next() {
        let p = param.first().copied().unwrap_or(0);

        // Handle extended color codes: 38 or 48 with sub-parameters.
        if p == 38 || p == 48 {
            // First check if the sub-params are embedded in the same parameter
            // (e.g. `38:2:r:g:b` or `38:5:n`).
            if param.len() >= 3 {
                // Sub-params present in the same param slot.
                let mode = param.get(1).copied().unwrap_or(0);
                if mode == 5 && param.len() >= 3 {
                    let idx = param.get(2).copied().unwrap_or(0) as u8;
                    set_extended_color(term, p, ExtColor::Indexed(idx));
                } else if mode == 2 && param.len() >= 5 {
                    let r = param.get(2).copied().unwrap_or(0) as u8;
                    let g = param.get(3).copied().unwrap_or(0) as u8;
                    let b = param.get(4).copied().unwrap_or(0) as u8;
                    set_extended_color(term, p, ExtColor::Rgb(r, g, b));
                }
                continue;
            }
            // Otherwise read from subsequent top-level params.
            if let Some(next) = iter.next() {
                let mode = next.first().copied().unwrap_or(0);
                if mode == 5 {
                    if let Some(idx_param) = iter.next() {
                        let idx = idx_param.first().copied().unwrap_or(0) as u8;
                        set_extended_color(term, p, ExtColor::Indexed(idx));
                    }
                } else if mode == 2 {
                    let r_p = iter.next().map_or(0, |x| x.first().copied().unwrap_or(0)) as u8;
                    let g_p = iter.next().map_or(0, |x| x.first().copied().unwrap_or(0)) as u8;
                    let b_p = iter.next().map_or(0, |x| x.first().copied().unwrap_or(0)) as u8;
                    set_extended_color(term, p, ExtColor::Rgb(r_p, g_p, b_p));
                }
            }
            continue;
        }

        term.apply_sgr(p);
    }
}

#[derive(Clone, Copy)]
enum ExtColor {
    Indexed(u8),
    Rgb(u8, u8, u8),
}

fn set_extended_color(term: &mut Terminal, sgr_code: u16, color: ExtColor) {
    use crate::grid::Color;
    let c = match color {
        ExtColor::Indexed(n) => Color::Indexed(n),
        ExtColor::Rgb(r, g, b) => Color::Rgb { r, g, b },
    };
    if sgr_code == 38 {
        term.grid.cursor.template.fg = c;
    } else {
        term.grid.cursor.template.bg = c;
    }
}

// ---------------------------------------------------------------------------
// Mode setters
// ---------------------------------------------------------------------------

fn set_private_mode(term: &mut Terminal, param: u16, set: bool) {
    match param {
        1 => toggle_mode(term, TermMode::APP_CURSOR, set),
        7 => toggle_mode(term, TermMode::LINE_WRAP, set),
        9 | 1000 => toggle_mode(term, TermMode::MOUSE_REPORT_CLICK, set), // X10 / X11 mouse
        12 => {
            term.cursor_blinking = set;
        }
        25 => toggle_mode(term, TermMode::SHOW_CURSOR, set),
        1002 => toggle_mode(term, TermMode::MOUSE_REPORT_DRAG, set), // button-event
        1003 => toggle_mode(term, TermMode::MOUSE_REPORT_MOTION, set), // any-event
        1005 => toggle_mode(term, TermMode::UTF8_MOUSE, set),        // UTF-8 mode
        1006 => toggle_mode(term, TermMode::SGR_MOUSE, set),         // SGR extended
        1049 => {
            // Only swap if we're actually changing state.
            let currently_alt = term.mode.contains(TermMode::ALT_SCREEN);
            if set != currently_alt {
                term.swap_alt_screen();
            }
        }
        2004 => toggle_mode(term, TermMode::BRACKETED_PASTE, set),
        _ => {}
    }
}

fn set_ansi_mode(term: &mut Terminal, param: u16, set: bool) {
    if param == 20 {
        toggle_mode(term, TermMode::LINE_FEED_NEW_LINE, set);
    }
}

fn toggle_mode(term: &mut Terminal, flag: TermMode, set: bool) {
    if set {
        term.mode.insert(flag);
    } else {
        term.mode.remove(flag);
    }
}

// ---------------------------------------------------------------------------
// Parameter helpers
// ---------------------------------------------------------------------------

/// Return the first top-level parameter value, defaulting to `default` if
/// missing or zero.
fn first_param(params: &Params, default: usize) -> usize {
    params
        .iter()
        .next()
        .and_then(|p| p.first().copied())
        .map_or(default as u16, |v| if v == 0 { default as u16 } else { v }) as usize
}

/// `pending_writes` にデータを追加する。`MAX_PENDING_WRITES` を超える場合は破棄する。
fn write_response(term: &mut Terminal, data: &[u8]) {
    if term.pending_writes.len().saturating_add(data.len()) <= MAX_PENDING_WRITES {
        term.pending_writes.extend_from_slice(data);
    }
}

/// Return the Nth (0-indexed) top-level parameter, defaulting to `default`.
fn nth_param(params: &Params, n: usize, default: usize) -> usize {
    params
        .iter()
        .nth(n)
        .and_then(|p| p.first().copied())
        .map_or(default as u16, |v| if v == 0 { default as u16 } else { v }) as usize
}
