//! VTE parser integration and Terminal state machine.
//!
//! [`Terminal`] holds a [`Grid<Cell>`] and implements [`vte::Perform`] to
//! mutate it in response to escape sequences.  [`Processor`] wraps a
//! [`vte::Parser`] to feed raw bytes into a [`Terminal`].

pub mod handler;

use std::ops::Range;

use bitflags::bitflags;
use vte::Perform;

use crate::grid::{Cell, CellFlags, Color, Dimensions, Grid, GridCell, NamedColor};
use crate::index::{Column, Line, Point};

/// `pending_writes` バッファの最大サイズ（バイト）。
/// 悪意あるプログラムが大量の DA/DSR/CPR リクエストを送信してメモリを枯渇させる
/// ことを防ぐ。超過分は破棄される。
const MAX_PENDING_WRITES: usize = 4096;

// ---------------------------------------------------------------------------
// CursorStyle
// ---------------------------------------------------------------------------

/// カーソルの表示形状。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CursorStyle {
    /// ブロックカーソル（■）。
    #[default]
    Block,
    /// アンダーラインカーソル（_）。
    Underline,
    /// バーカーソル（|）。
    Bar,
}

// ---------------------------------------------------------------------------
// TermMode
// ---------------------------------------------------------------------------

bitflags! {
    /// Active terminal mode flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct TermMode: u32 {
        /// Show the cursor (DECTCEM — default ON).
        const SHOW_CURSOR      = 0b0000_0000_0001;
        /// Automatic line-wrap (DECAWM — default ON).
        const LINE_WRAP        = 0b0000_0000_0010;
        /// Origin mode (DECOM).
        const ORIGIN           = 0b0000_0000_0100;
        /// Insert mode.
        const INSERT           = 0b0000_0000_1000;
        /// Application cursor keys.
        const APP_CURSOR       = 0b0000_0001_0000;
        /// Application keypad mode.
        const APP_KEYPAD       = 0b0000_0010_0000;
        /// Alternate screen is active.
        const ALT_SCREEN       = 0b0000_0100_0000;
        /// Bracketed paste mode.
        const BRACKETED_PASTE  = 0b0000_1000_0000;
        /// LF also moves cursor to column 0 (LNM).
        const LINE_FEED_NEW_LINE = 0b0001_0000_0000;
        /// X10/X11 mouse click reporting (?9 / ?1000).
        const MOUSE_REPORT_CLICK  = 0b0010_0000_0000;
        /// Button-event mouse tracking (?1002).
        const MOUSE_REPORT_DRAG   = 0b0100_0000_0000;
        /// Any-event mouse tracking (?1003).
        const MOUSE_REPORT_MOTION = 0b1000_0000_0000;
        /// SGR extended mouse coordinates (?1006).
        const SGR_MOUSE           = 0b0001_0000_0000_0000;
        /// UTF-8 mouse mode (?1005).
        const UTF8_MOUSE          = 0b0010_0000_0000_0000;
    }
}

impl TermMode {
    /// Default modes active when a terminal is first created.
    fn defaults() -> Self {
        Self::SHOW_CURSOR | Self::LINE_WRAP
    }
}

// ---------------------------------------------------------------------------
// Terminal
// ---------------------------------------------------------------------------

/// Terminal emulator state machine.
///
/// Owns a primary and an alternate [`Grid<Cell>`], mode flags, scroll region,
/// tab stops, and the optional window title set via OSC 0/2.
pub struct Terminal {
    /// Primary grid (active when `ALT_SCREEN` is not set).
    pub(super) grid: Grid<Cell>,
    /// Alternate grid (active when `ALT_SCREEN` is set).
    pub(super) inactive_grid: Grid<Cell>,
    /// Active mode flags.
    pub(super) mode: TermMode,
    /// Scroll region expressed as `top..bottom` (0-indexed, inclusive bottom).
    pub(super) scroll_region: Range<usize>,
    /// Tab-stop bitmask: `tabs[col]` is `true` when column `col` is a tab stop.
    pub(super) tabs: Vec<bool>,
    /// Window title, set via `OSC 0` / `OSC 2`.
    pub(super) title: Option<String>,
    /// 応答バイト列バッファ（DA/DSR/CPR等のPTYへの応答）。
    /// `MAX_PENDING_WRITES` バイトを超えた応答は破棄される。
    pub(super) pending_writes: Vec<u8>,
    /// カーソルスタイル。
    pub(super) cursor_style: CursorStyle,
    /// カーソル点滅が有効か。
    pub(super) cursor_blinking: bool,
    /// BEL (0x07) を受信したか。
    pub(super) bell_pending: bool,
    /// OSC 52 クリップボード書き込み要求。
    pub(super) clipboard_write_pending: Option<String>,
}

impl Terminal {
    /// Create a new [`Terminal`] with `lines` rows and `columns` columns.
    ///
    /// `max_scroll_limit` controls how many lines of history the primary grid
    /// retains.  The alternate grid never retains history.
    pub fn new(lines: usize, columns: usize, max_scroll_limit: usize) -> Self {
        let tabs = build_tabs(columns);
        Self {
            grid: Grid::new(lines, columns, max_scroll_limit),
            inactive_grid: Grid::new(lines, columns, 0),
            mode: TermMode::defaults(),
            scroll_region: 0..lines,
            tabs,
            title: None,
            pending_writes: Vec::new(),
            cursor_style: CursorStyle::default(),
            cursor_blinking: false,
            bell_pending: false,
            clipboard_write_pending: None,
        }
    }

    /// Read-only reference to the active grid.
    pub fn grid(&self) -> &Grid<Cell> {
        &self.grid
    }

    /// Mutable reference to the active grid.
    pub fn grid_mut(&mut self) -> &mut Grid<Cell> {
        &mut self.grid
    }

    /// Currently active terminal mode flags.
    pub fn mode(&self) -> TermMode {
        self.mode
    }

    /// Window title as set by the most recent `OSC 0/2` sequence, or `None`.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// 蓄積された応答バイト列を取り出す。空なら `None` を返す。
    pub fn drain_pending_writes(&mut self) -> Option<Vec<u8>> {
        if self.pending_writes.is_empty() {
            None
        } else {
            Some(std::mem::take(&mut self.pending_writes))
        }
    }

    /// カーソルスタイルを返す。
    pub fn cursor_style(&self) -> CursorStyle {
        self.cursor_style
    }

    /// カーソル点滅が有効かを返す。
    pub fn cursor_blinking(&self) -> bool {
        self.cursor_blinking
    }

    /// いずれかのマウス報告モードがアクティブかどうかを返す。
    pub fn mouse_mode_active(&self) -> bool {
        self.mode.intersects(
            TermMode::MOUSE_REPORT_CLICK
                | TermMode::MOUSE_REPORT_DRAG
                | TermMode::MOUSE_REPORT_MOTION,
        )
    }

    /// ベルが鳴った（BEL受信）かどうかを確認し、フラグをリセットする。
    pub fn take_bell(&mut self) -> bool {
        std::mem::take(&mut self.bell_pending)
    }

    /// OSC 52 クリップボード書き込み要求を取り出す。
    ///
    /// 呼び出し後はフィールドが `None` になる（take セマンティクス）。
    pub fn take_clipboard_write(&mut self) -> Option<String> {
        self.clipboard_write_pending.take()
    }

    /// Resize the terminal to `lines` rows and `columns` columns.
    ///
    /// Both grids are resized; the scroll region is reset to the full viewport;
    /// tab stops are rebuilt.
    pub fn resize(&mut self, lines: usize, columns: usize) {
        self.grid.resize(lines, columns);
        self.inactive_grid.resize(lines, columns);
        self.scroll_region = 0..lines;
        self.tabs = build_tabs(columns);
    }

    // -----------------------------------------------------------------------
    // Internal helpers (pub(super) so handler.rs can call them)
    // -----------------------------------------------------------------------

    /// Move the cursor to an absolute viewport position, clamping to bounds.
    pub(super) fn set_cursor(&mut self, line: usize, column: usize) {
        let max_line = self.grid.screen_lines().saturating_sub(1);
        let max_col = self.grid.columns().saturating_sub(1);
        self.grid.cursor.point.line = Line(i32::try_from(line.min(max_line)).unwrap_or(i32::MAX));
        self.grid.cursor.point.column = Column(column.min(max_col));
        self.grid.cursor.input_needs_wrap = false;
    }

    /// Perform a linefeed: scroll or move the cursor down.
    ///
    /// When the cursor is at the bottom of the scroll region, `scroll_up` is
    /// called; otherwise the cursor moves down one line.
    pub(super) fn linefeed(&mut self) {
        let cur_line = self.grid.cursor.point.line.as_viewport_idx();
        if cur_line + 1 == self.scroll_region.end {
            self.grid.scroll_up(self.scroll_region.clone(), 1);
        } else if cur_line + 1 < self.grid.screen_lines() {
            self.grid.cursor.point.line += 1;
        }
    }

    /// Carriage return: move cursor to column 0.
    pub(super) fn carriage_return(&mut self) {
        self.grid.cursor.point.column = Column(0);
        self.grid.cursor.input_needs_wrap = false;
    }

    /// Swap the primary and alternate grids.
    ///
    /// When entering alt-screen: saves the primary cursor and clears the new
    /// (alternate) viewport.  When leaving: restores the saved cursor.
    pub(super) fn swap_alt_screen(&mut self) {
        std::mem::swap(&mut self.grid, &mut self.inactive_grid);
        self.mode.toggle(TermMode::ALT_SCREEN);

        if self.mode.contains(TermMode::ALT_SCREEN) {
            // Entering alt screen: clear it.
            self.grid.clear_viewport();
            self.grid.cursor = crate::grid::Cursor::default();
        }
        // Leaving: the primary cursor was preserved in inactive_grid.cursor —
        // the swap already restored it.
    }

    /// Erase a span of cells in the *current viewport* from `start` to `end`
    /// (inclusive) using the cursor template for colour attributes.
    pub(super) fn erase_cells(&mut self, start: Point, end: Point) {
        // Guard against inverted ranges, which would cause the loop to run
        // indefinitely or wrap around into out-of-range rows.
        if start > end {
            return;
        }
        let template = self.grid.cursor.template.clone();
        let cols = self.grid.columns();
        let mut pt = start;
        loop {
            self.grid[pt].reset(&template);
            if pt == end {
                break;
            }
            let next_col = pt.column.0 + 1;
            if next_col >= cols {
                pt.column = Column(0);
                pt.line += 1;
                if pt.line.as_viewport_idx() >= self.grid.screen_lines() {
                    break;
                }
            } else {
                pt.column = Column(next_col);
            }
        }
    }

    /// Erase from the cursor to the end of the current line.
    pub(super) fn erase_to_eol(&mut self) {
        let start = self.grid.cursor.point;
        let end = Point::new(start.line, Column(self.grid.columns().saturating_sub(1)));
        self.erase_cells(start, end);
    }

    /// Erase from the start of the current line to the cursor (inclusive).
    pub(super) fn erase_to_bol(&mut self) {
        let cur = self.grid.cursor.point;
        let start = Point::new(cur.line, Column(0));
        self.erase_cells(start, cur);
    }

    /// Erase the entire current line.
    pub(super) fn erase_line(&mut self) {
        let line = self.grid.cursor.point.line;
        let start = Point::new(line, Column(0));
        let end = Point::new(line, Column(self.grid.columns().saturating_sub(1)));
        self.erase_cells(start, end);
    }

    /// Reset the cursor template to defaults.
    pub(super) fn reset_sgr(&mut self) {
        self.grid.cursor.template = Cell::default();
    }

    /// Apply a single parsed SGR parameter value to the cursor template.
    pub(super) fn apply_sgr(&mut self, param: u16) {
        let tmpl = &mut self.grid.cursor.template;
        match param {
            0 => self.reset_sgr(),
            1 => tmpl.flags |= CellFlags::BOLD,
            3 => tmpl.flags |= CellFlags::ITALIC,
            4 => tmpl.flags |= CellFlags::UNDERLINE,
            7 => tmpl.flags |= CellFlags::INVERSE,
            8 => tmpl.flags |= CellFlags::HIDDEN,
            9 => tmpl.flags |= CellFlags::STRIKEOUT,
            21 => tmpl.flags.remove(CellFlags::BOLD),
            22 => tmpl.flags.remove(CellFlags::BOLD | CellFlags::DIM),
            23 => tmpl.flags.remove(CellFlags::ITALIC),
            24 => tmpl.flags.remove(CellFlags::UNDERLINE),
            27 => tmpl.flags.remove(CellFlags::INVERSE),
            28 => tmpl.flags.remove(CellFlags::HIDDEN),
            29 => tmpl.flags.remove(CellFlags::STRIKEOUT),
            // Foreground: standard colors (30-37)
            30 => tmpl.fg = Color::Named(NamedColor::Black),
            31 => tmpl.fg = Color::Named(NamedColor::Red),
            32 => tmpl.fg = Color::Named(NamedColor::Green),
            33 => tmpl.fg = Color::Named(NamedColor::Yellow),
            34 => tmpl.fg = Color::Named(NamedColor::Blue),
            35 => tmpl.fg = Color::Named(NamedColor::Magenta),
            36 => tmpl.fg = Color::Named(NamedColor::Cyan),
            37 => tmpl.fg = Color::Named(NamedColor::White),
            39 => tmpl.fg = Color::Named(NamedColor::Foreground),
            // Background: standard colors (40-47)
            40 => tmpl.bg = Color::Named(NamedColor::Black),
            41 => tmpl.bg = Color::Named(NamedColor::Red),
            42 => tmpl.bg = Color::Named(NamedColor::Green),
            43 => tmpl.bg = Color::Named(NamedColor::Yellow),
            44 => tmpl.bg = Color::Named(NamedColor::Blue),
            45 => tmpl.bg = Color::Named(NamedColor::Magenta),
            46 => tmpl.bg = Color::Named(NamedColor::Cyan),
            47 => tmpl.bg = Color::Named(NamedColor::White),
            49 => tmpl.bg = Color::Named(NamedColor::Background),
            // Foreground: bright colors (90-97)
            90 => tmpl.fg = Color::Named(NamedColor::BrightBlack),
            91 => tmpl.fg = Color::Named(NamedColor::BrightRed),
            92 => tmpl.fg = Color::Named(NamedColor::BrightGreen),
            93 => tmpl.fg = Color::Named(NamedColor::BrightYellow),
            94 => tmpl.fg = Color::Named(NamedColor::BrightBlue),
            95 => tmpl.fg = Color::Named(NamedColor::BrightMagenta),
            96 => tmpl.fg = Color::Named(NamedColor::BrightCyan),
            97 => tmpl.fg = Color::Named(NamedColor::BrightWhite),
            // Background: bright colors (100-107)
            100 => tmpl.bg = Color::Named(NamedColor::BrightBlack),
            101 => tmpl.bg = Color::Named(NamedColor::BrightRed),
            102 => tmpl.bg = Color::Named(NamedColor::BrightGreen),
            103 => tmpl.bg = Color::Named(NamedColor::BrightYellow),
            104 => tmpl.bg = Color::Named(NamedColor::BrightBlue),
            105 => tmpl.bg = Color::Named(NamedColor::BrightMagenta),
            106 => tmpl.bg = Color::Named(NamedColor::BrightCyan),
            107 => tmpl.bg = Color::Named(NamedColor::BrightWhite),
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// vte::Perform implementation
// ---------------------------------------------------------------------------

impl Perform for Terminal {
    fn print(&mut self, c: char) {
        // If we need to wrap, do so before placing the character.
        if self.grid.cursor.input_needs_wrap && self.mode.contains(TermMode::LINE_WRAP) {
            // Mark the current cell as wrapped.
            let cur = self.grid.cursor.point;
            self.grid[cur].flags |= CellFlags::WRAPLINE;
            self.linefeed();
            self.carriage_return();
        }

        let width = unicode_width::UnicodeWidthChar::width(c).unwrap_or(1);

        let col = self.grid.cursor.point.column.0;
        let cols = self.grid.columns();

        if width == 2 && col + 1 >= cols {
            // Wide character doesn't fit: fill with spaces and wrap.
            let cur = self.grid.cursor.point;
            let tmpl = self.grid.cursor.template.clone();
            self.grid[cur].reset(&tmpl);
            if self.mode.contains(TermMode::LINE_WRAP) {
                self.grid[cur].flags |= CellFlags::WRAPLINE;
                self.linefeed();
                self.carriage_return();
            } else {
                return;
            }
        }

        // Write the character.
        {
            let tmpl = self.grid.cursor.template.clone();
            let cell = self.grid.cursor_cell();
            cell.c = c;
            cell.fg = tmpl.fg;
            cell.bg = tmpl.bg;
            cell.flags = tmpl.flags;
            if width == 2 {
                cell.flags |= CellFlags::WIDE_CHAR;
            }
        }

        let cur_col = self.grid.cursor.point.column.0;

        if width == 2 {
            // Place spacer in the next cell.
            let spacer_col = cur_col + 1;
            if spacer_col < cols {
                let spacer_point = Point::new(self.grid.cursor.point.line, Column(spacer_col));
                let tmpl = self.grid.cursor.template.clone();
                let spacer = &mut self.grid[spacer_point];
                spacer.reset(&tmpl);
                spacer.flags |= CellFlags::WIDE_CHAR_SPACER;
            }
            // Advance cursor by 2.
            let next_col = cur_col + 2;
            if next_col >= cols {
                self.grid.cursor.input_needs_wrap = true;
            } else {
                self.grid.cursor.point.column = Column(next_col);
            }
        } else {
            // Advance cursor by 1.
            let next_col = cur_col + 1;
            if next_col >= cols {
                self.grid.cursor.input_needs_wrap = true;
            } else {
                self.grid.cursor.point.column = Column(next_col);
            }
        }
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            // BEL
            0x07 => {
                self.bell_pending = true;
            }
            // Backspace
            0x08 => {
                let col = self.grid.cursor.point.column.0;
                if col > 0 {
                    self.grid.cursor.point.column = Column(col - 1);
                }
                self.grid.cursor.input_needs_wrap = false;
            }
            // Horizontal tab
            0x09 => {
                let cur_col = self.grid.cursor.point.column.0;
                let cols = self.grid.columns();
                // Find the next tab stop after the current column.
                let next = (cur_col + 1..cols).find(|&c| self.tabs[c]);
                let dest = next.unwrap_or(cols.saturating_sub(1));
                self.grid.cursor.point.column = Column(dest);
                self.grid.cursor.input_needs_wrap = false;
            }
            // LF, VT, FF
            0x0A..=0x0C => {
                self.linefeed();
                if self.mode.contains(TermMode::LINE_FEED_NEW_LINE) {
                    self.carriage_return();
                }
            }
            // CR
            0x0D => {
                self.carriage_return();
            }
            _ => {}
        }
    }

    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        handler::csi_dispatch(self, params, intermediates, action);
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, byte: u8) {
        handler::esc_dispatch(self, intermediates, byte);
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        // OSC 0 or OSC 2: set window title.
        if params.len() >= 2 && (params[0] == b"0" || params[0] == b"2") {
            // Limit title length to prevent memory exhaustion from malicious
            // sequences (e.g. OSC 0;<multi-MB string>BEL).
            const MAX_TITLE_BYTES: usize = 4096;
            let raw = params[1];
            let capped = if raw.len() > MAX_TITLE_BYTES { &raw[..MAX_TITLE_BYTES] } else { raw };
            if let Ok(title) = std::str::from_utf8(capped) {
                self.title = Some(title.to_owned());
            }
        }
        // OSC 52: クリップボード操作。
        // 書き込み: \e]52;c;<base64_data>BEL
        // 読み取り: \e]52;c;?BEL — セキュリティのため応答しない
        if params.len() >= 3 && params[0] == b"52" {
            // パラメータ 1: クリップボード選択 ("c" = 通常クリップボード)
            // パラメータ 2: base64 データ または "?"
            let data = params[2];
            if data == b"?" {
                // 読み取り要求: 悪意あるアプリによる窃取を防ぐため応答しない
            } else {
                // base64 デコード
                if let Some(text) = decode_base64(data) {
                    // クリップボード書き込み量の上限: 1 MiB
                    const MAX_CLIPBOARD_BYTES: usize = 1024 * 1024;
                    if text.len() <= MAX_CLIPBOARD_BYTES {
                        self.clipboard_write_pending = Some(text);
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Processor
// ---------------------------------------------------------------------------

/// Feeds raw bytes through the VTE parser into a [`Terminal`].
pub struct Processor {
    parser: vte::Parser,
}

impl Processor {
    /// Create a new `Processor` with a fresh VTE parser.
    pub fn new() -> Self {
        Self { parser: vte::Parser::new() }
    }

    /// Advance the parser with `bytes`, dispatching each action to `terminal`.
    pub fn advance(&mut self, terminal: &mut Terminal, bytes: &[u8]) {
        for &byte in bytes {
            self.parser.advance(terminal, byte);
        }
    }
}

impl Default for Processor {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a tab-stop vector for `columns` columns (stops every 8 characters).
fn build_tabs(columns: usize) -> Vec<bool> {
    (0..columns).map(|c| c != 0 && c % 8 == 0).collect()
}

/// 標準的な Base64（RFC 4648）をデコードして UTF-8 文字列を返す。
///
/// デコードに失敗した場合は `None` を返す。
fn decode_base64(input: &[u8]) -> Option<String> {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut decode_map = [0xffu8; 256];
    for (i, &b) in TABLE.iter().enumerate() {
        decode_map[b as usize] = i as u8;
    }

    let mut output: Vec<u8> = Vec::with_capacity(input.len() / 4 * 3);
    let mut buf = 0u32;
    let mut bits = 0u8;

    for &b in input {
        if b == b'=' {
            break;
        }
        if b == b'\n' || b == b'\r' || b == b' ' {
            continue;
        }
        let val = decode_map[b as usize];
        if val == 0xff {
            return None; // 不正な文字
        }
        buf = (buf << 6) | u32::from(val);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            output.push((buf >> bits) as u8);
        }
    }

    String::from_utf8(output).ok()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::{Column, Line, Point};

    fn make_proc_term(lines: usize, cols: usize) -> (Processor, Terminal) {
        (Processor::new(), Terminal::new(lines, cols, 100))
    }

    // 1. print テスト: "Hello" を送り込んでセルを確認する
    #[test]
    fn print_hello() {
        let (mut proc, mut term) = make_proc_term(24, 80);
        proc.advance(&mut term, b"Hello");
        assert_eq!(term.grid()[Point::new(Line(0), Column(0))].c, 'H');
        assert_eq!(term.grid()[Point::new(Line(0), Column(1))].c, 'e');
        assert_eq!(term.grid()[Point::new(Line(0), Column(4))].c, 'o');
        // cursor is at column 5
        assert_eq!(term.grid().cursor.point.column, Column(5));
    }

    // 2. 改行テスト: LF+CR でカーソルが次の行頭に移動する
    #[test]
    fn linefeed_and_cr() {
        let (mut proc, mut term) = make_proc_term(24, 80);
        proc.advance(&mut term, b"A\r\nB");
        // 'A' at (0,0), 'B' at (1,0)
        assert_eq!(term.grid()[Point::new(Line(0), Column(0))].c, 'A');
        assert_eq!(term.grid()[Point::new(Line(1), Column(0))].c, 'B');
        assert_eq!(term.grid().cursor.point.line, Line(1));
        assert_eq!(term.grid().cursor.point.column, Column(1));
    }

    // 3. カーソル移動テスト: CUP (ESC[row;colH)
    #[test]
    fn cursor_position() {
        let (mut proc, mut term) = make_proc_term(24, 80);
        // ESC [ 5 ; 10 H  →  line=4, col=9 (1-based → 0-based)
        proc.advance(&mut term, b"\x1b[5;10H");
        assert_eq!(term.grid().cursor.point.line, Line(4));
        assert_eq!(term.grid().cursor.point.column, Column(9));
    }

    // 4. SGR テスト: SGR 1 (BOLD) → print → セルの flags に BOLD が含まれる
    #[test]
    fn sgr_bold() {
        let (mut proc, mut term) = make_proc_term(24, 80);
        proc.advance(&mut term, b"\x1b[1mX");
        let cell = &term.grid()[Point::new(Line(0), Column(0))];
        assert_eq!(cell.c, 'X');
        assert!(cell.flags.contains(CellFlags::BOLD));
    }

    // 5. 画面消去テスト: ED 2 (ESC[2J) で全画面消去
    #[test]
    fn erase_display_all() {
        let (mut proc, mut term) = make_proc_term(24, 80);
        proc.advance(&mut term, b"Hello");
        proc.advance(&mut term, b"\x1b[2J");
        // Entire screen should be blank.
        for line in 0..24_i32 {
            for col in 0..80 {
                let c = term.grid()[Point::new(Line(line), Column(col))].c;
                assert_eq!(c, ' ', "expected space at ({line},{col})");
            }
        }
    }

    // 6. スクロールテスト: 画面下端で LF → scroll_up が発生
    #[test]
    fn scroll_at_bottom() {
        let (mut proc, mut term) = make_proc_term(5, 10);
        // Fill all 5 lines, then add an extra LF.
        proc.advance(&mut term, b"A\r\nB\r\nC\r\nD\r\nE");
        let history_before = term.grid().history_size();
        proc.advance(&mut term, b"\r\n");
        // After LF from line 4 (0-indexed bottom), scroll_up should have fired.
        assert!(
            term.grid().history_size() > history_before,
            "expected scroll_up to push a line into history"
        );
    }

    // 7. alt screen テスト: 切替→復帰でプライマリ画面が復元される
    #[test]
    fn alt_screen_roundtrip() {
        let (mut proc, mut term) = make_proc_term(24, 80);
        // Write to primary.
        proc.advance(&mut term, b"Primary");
        let primary_char = term.grid()[Point::new(Line(0), Column(0))].c;
        assert_eq!(primary_char, 'P');

        // Switch to alt screen.
        proc.advance(&mut term, b"\x1b[?1049h");
        assert!(term.mode().contains(TermMode::ALT_SCREEN));
        // Alt screen should be blank.
        assert_eq!(term.grid()[Point::new(Line(0), Column(0))].c, ' ');

        // Write something on alt screen.
        proc.advance(&mut term, b"Alt");

        // Switch back.
        proc.advance(&mut term, b"\x1b[?1049l");
        assert!(!term.mode().contains(TermMode::ALT_SCREEN));
        // Primary content should be restored.
        assert_eq!(term.grid()[Point::new(Line(0), Column(0))].c, 'P');
    }

    // 8. Processor テスト: バイト列を渡して Grid 状態を確認
    #[test]
    fn processor_advance_complex() {
        let (mut proc, mut term) = make_proc_term(24, 80);
        // Write "AB", move cursor to (0,0), overwrite with "CD".
        proc.advance(&mut term, b"AB\x1b[1;1HCD");
        assert_eq!(term.grid()[Point::new(Line(0), Column(0))].c, 'C');
        assert_eq!(term.grid()[Point::new(Line(0), Column(1))].c, 'D');
    }

    // Additional: SGR reset
    #[test]
    fn sgr_reset() {
        let (mut proc, mut term) = make_proc_term(24, 80);
        proc.advance(&mut term, b"\x1b[1;3m"); // BOLD + ITALIC
        assert!(term.grid().cursor.template.flags.contains(CellFlags::BOLD));
        proc.advance(&mut term, b"\x1b[0m"); // reset
        assert!(!term.grid().cursor.template.flags.contains(CellFlags::BOLD));
        assert!(!term.grid().cursor.template.flags.contains(CellFlags::ITALIC));
    }

    // Additional: cursor up / down / left / right
    #[test]
    fn cursor_movements() {
        let (mut proc, mut term) = make_proc_term(24, 80);
        proc.advance(&mut term, b"\x1b[10;20H"); // line=9, col=19
        proc.advance(&mut term, b"\x1b[2A"); // up 2 → line=7
        assert_eq!(term.grid().cursor.point.line, Line(7));
        proc.advance(&mut term, b"\x1b[3B"); // down 3 → line=10
        assert_eq!(term.grid().cursor.point.line, Line(10));
        proc.advance(&mut term, b"\x1b[5C"); // right 5 → col=24
        assert_eq!(term.grid().cursor.point.column, Column(24));
        proc.advance(&mut term, b"\x1b[10D"); // left 10 → col=14
        assert_eq!(term.grid().cursor.point.column, Column(14));
    }

    // Additional: erase line
    #[test]
    fn erase_line() {
        let (mut proc, mut term) = make_proc_term(24, 80);
        proc.advance(&mut term, b"Hello World");
        proc.advance(&mut term, b"\x1b[2K"); // erase whole line
        for col in 0..80 {
            assert_eq!(term.grid()[Point::new(Line(0), Column(col))].c, ' ', "col {col} not blank");
        }
    }

    // Additional: OSC title length limit (defense-in-depth)
    #[test]
    fn osc_title_capped() {
        let (mut proc, mut term) = make_proc_term(24, 80);
        // Build an OSC 0 sequence with a long title.
        // Note: vte 0.13's internal buffer truncates OSC params at ~1024 bytes,
        // so the title that reaches our handler is already capped by the parser.
        // Our MAX_TITLE_BYTES (4096) is defense-in-depth for future vte changes.
        let mut seq = Vec::new();
        seq.extend_from_slice(b"\x1b]0;");
        seq.extend(std::iter::repeat_n(b'A', 5000));
        seq.push(0x07); // BEL
        proc.advance(&mut term, &seq);
        let title = term.title().expect("title should be set");
        // The title must be bounded (vte caps at ~1024, our limit at 4096).
        assert!(title.len() <= 4096);
        assert!(!title.is_empty());
    }

    // Test our OSC handler directly to verify the 4096 cap logic.
    #[test]
    fn osc_title_direct_cap_at_4096() {
        let mut term = Terminal::new(24, 80, 100);
        // Simulate OSC dispatch with a payload exceeding 4096 bytes,
        // bypassing vte's parser buffer limit.
        let long_title: Vec<u8> = std::iter::repeat_n(b'B', 5000).collect();
        term.osc_dispatch(&[b"0", &long_title], false);
        let title = term.title().expect("title should be set");
        assert_eq!(title.len(), 4096);
    }

    // Additional: scroll region (DECSTBM)
    #[test]
    fn decstbm_sets_scroll_region() {
        let (mut proc, mut term) = make_proc_term(24, 80);
        proc.advance(&mut term, b"\x1b[5;20r");
        assert_eq!(term.scroll_region, 4..20);
    }

    // Additional: IL/DL
    #[test]
    fn insert_and_delete_lines() {
        let (mut proc, mut term) = make_proc_term(10, 20);
        // Write to line 0.
        proc.advance(&mut term, b"Line0");
        proc.advance(&mut term, b"\r\nLine1");
        // Cursor is now at (1, 5). Insert 1 line at current line.
        proc.advance(&mut term, b"\x1b[1L");
        // "Line0" should still be at line 0.
        assert_eq!(term.grid()[Point::new(Line(0), Column(0))].c, 'L');
        // Line 1 should now be blank (inserted line).
        assert_eq!(term.grid()[Point::new(Line(1), Column(0))].c, ' ');
        // Former line 1 ("Line1") should be at line 2.
        assert_eq!(term.grid()[Point::new(Line(2), Column(0))].c, 'L');

        // Delete the inserted blank line (cursor is still at line 1).
        proc.advance(&mut term, b"\x1b[1;1H\x1b[1;1H"); // reposition for clarity
        proc.advance(&mut term, b"\x1b[2;1H"); // move to line 2 (1-based = line index 1)
        proc.advance(&mut term, b"\x1b[1M");
        // Line 1 should now be "Line1" again.
        assert_eq!(term.grid()[Point::new(Line(1), Column(0))].c, 'L');
    }

    // CJK: 全角文字が2セル幅で配置される
    #[test]
    fn cjk_wide_char() {
        let (mut proc, mut term) = make_proc_term(24, 80);
        // 日本語の "あ" は全角（幅2）
        proc.advance(&mut term, "あ".as_bytes());
        let cell0 = &term.grid()[Point::new(Line(0), Column(0))];
        assert_eq!(cell0.c, 'あ');
        assert!(cell0.flags.contains(CellFlags::WIDE_CHAR));

        let cell1 = &term.grid()[Point::new(Line(0), Column(1))];
        assert!(cell1.flags.contains(CellFlags::WIDE_CHAR_SPACER));

        // カーソルは列2に進んでいる
        assert_eq!(term.grid().cursor.point.column, Column(2));
    }

    // CJK: 行末で全角文字がはみ出す場合のラップ
    #[test]
    fn cjk_wrap_at_line_end() {
        let (mut proc, mut term) = make_proc_term(24, 10);
        // 列0〜8に9文字書いて、列9に全角文字を書く（はみ出すのでラップ）
        proc.advance(&mut term, b"123456789");
        proc.advance(&mut term, "あ".as_bytes());

        // "あ" は次の行の先頭に配置される
        let cell = &term.grid()[Point::new(Line(1), Column(0))];
        assert_eq!(cell.c, 'あ');
        assert!(cell.flags.contains(CellFlags::WIDE_CHAR));
    }

    // CJK: unicode_width による幅判定
    #[test]
    fn cjk_unicode_width() {
        use unicode_width::UnicodeWidthChar;
        assert_eq!(UnicodeWidthChar::width('あ'), Some(2));
        assert_eq!(UnicodeWidthChar::width('漢'), Some(2));
        assert_eq!(UnicodeWidthChar::width('A'), Some(1));
        assert_eq!(UnicodeWidthChar::width('ｱ'), Some(1)); // 半角カタカナ
    }

    // DA1 応答テスト
    #[test]
    fn da1_response() {
        let (mut proc, mut term) = make_proc_term(24, 80);
        proc.advance(&mut term, b"\x1b[c");
        let response = term.drain_pending_writes().unwrap();
        assert_eq!(response, b"\x1b[?62;4c");
    }

    // DA2 応答テスト
    #[test]
    fn da2_response() {
        let (mut proc, mut term) = make_proc_term(24, 80);
        proc.advance(&mut term, b"\x1b[>c");
        let response = term.drain_pending_writes().unwrap();
        assert_eq!(response, b"\x1b[>0;0;0c");
    }

    // DSR 応答テスト
    #[test]
    fn dsr_status_ok() {
        let (mut proc, mut term) = make_proc_term(24, 80);
        proc.advance(&mut term, b"\x1b[5n");
        let response = term.drain_pending_writes().unwrap();
        assert_eq!(response, b"\x1b[0n");
    }

    // CPR 応答テスト
    #[test]
    fn cpr_cursor_position_report() {
        let (mut proc, mut term) = make_proc_term(24, 80);
        proc.advance(&mut term, b"\x1b[5;10H"); // move to (5,10) 1-based
        proc.advance(&mut term, b"\x1b[6n");
        let response = term.drain_pending_writes().unwrap();
        assert_eq!(response, b"\x1b[5;10R");
    }

    // DECSCUSR テスト
    #[test]
    fn decscusr_cursor_style() {
        let (mut proc, mut term) = make_proc_term(24, 80);
        // Bar (blinking)
        proc.advance(&mut term, b"\x1b[5 q");
        assert_eq!(term.cursor_style(), CursorStyle::Bar);
        assert!(term.cursor_blinking());
        // Underline (steady)
        proc.advance(&mut term, b"\x1b[4 q");
        assert_eq!(term.cursor_style(), CursorStyle::Underline);
        assert!(!term.cursor_blinking());
        // Block (blinking)
        proc.advance(&mut term, b"\x1b[1 q");
        assert_eq!(term.cursor_style(), CursorStyle::Block);
        assert!(term.cursor_blinking());
    }

    // マウスモード: CSI ? 1000 h で MOUSE_REPORT_CLICK がセットされる
    #[test]
    fn mouse_mode_click_set() {
        let (mut proc, mut term) = make_proc_term(24, 80);
        proc.advance(&mut term, b"\x1b[?1000h");
        assert!(term.mode().contains(TermMode::MOUSE_REPORT_CLICK));
        assert!(term.mouse_mode_active());
    }

    // マウスモード: CSI ? 1006 h で SGR_MOUSE がセットされる
    #[test]
    fn mouse_mode_sgr_set() {
        let (mut proc, mut term) = make_proc_term(24, 80);
        proc.advance(&mut term, b"\x1b[?1006h");
        assert!(term.mode().contains(TermMode::SGR_MOUSE));
    }

    // マウスモード: CSI ? 1000 l でリセットされる
    #[test]
    fn mouse_mode_click_reset() {
        let (mut proc, mut term) = make_proc_term(24, 80);
        proc.advance(&mut term, b"\x1b[?1000h");
        assert!(term.mode().contains(TermMode::MOUSE_REPORT_CLICK));
        proc.advance(&mut term, b"\x1b[?1000l");
        assert!(!term.mode().contains(TermMode::MOUSE_REPORT_CLICK));
        assert!(!term.mouse_mode_active());
    }

    // マウスモード: X10 (?9) / drag (?1002) / motion (?1003) も設定できる
    #[test]
    fn mouse_mode_variants() {
        let (mut proc, mut term) = make_proc_term(24, 80);
        proc.advance(&mut term, b"\x1b[?9h");
        assert!(term.mode().contains(TermMode::MOUSE_REPORT_CLICK));
        proc.advance(&mut term, b"\x1b[?9l");
        proc.advance(&mut term, b"\x1b[?1002h");
        assert!(term.mode().contains(TermMode::MOUSE_REPORT_DRAG));
        assert!(term.mouse_mode_active());
        proc.advance(&mut term, b"\x1b[?1003h");
        assert!(term.mode().contains(TermMode::MOUSE_REPORT_MOTION));
        proc.advance(&mut term, b"\x1b[?1005h");
        assert!(term.mode().contains(TermMode::UTF8_MOUSE));
    }

    // OSC 52: クリップボード書き込みテスト
    #[test]
    fn osc52_clipboard_write() {
        let mut term = Terminal::new(24, 80, 0);
        // "Hello" の Base64 は "SGVsbG8="
        term.osc_dispatch(&[b"52", b"c", b"SGVsbG8="], false);
        assert_eq!(term.take_clipboard_write(), Some("Hello".to_string()));
        // 2回目は None（take セマンティクス）
        assert_eq!(term.take_clipboard_write(), None);
    }

    #[test]
    fn osc52_clipboard_read_request_ignored() {
        let mut term = Terminal::new(24, 80, 0);
        // 読み取り要求 "?" は無視する
        term.osc_dispatch(&[b"52", b"c", b"?"], false);
        assert_eq!(term.take_clipboard_write(), None);
    }

    #[test]
    fn osc52_invalid_base64_ignored() {
        let mut term = Terminal::new(24, 80, 0);
        term.osc_dispatch(&[b"52", b"c", b"not!valid!base64!!!"], false);
        // 不正な文字は None
        assert_eq!(term.take_clipboard_write(), None);
    }

    #[test]
    fn decode_base64_basic() {
        // 内部ヘルパーを osc_dispatch 経由で間接テスト
        let mut term = Terminal::new(24, 80, 0);
        // "test" → "dGVzdA=="
        term.osc_dispatch(&[b"52", b"c", b"dGVzdA=="], false);
        assert_eq!(term.take_clipboard_write(), Some("test".to_string()));
    }

    // pending_writes サイズ制限テスト
    #[test]
    fn pending_writes_respects_max_limit() {
        let (mut proc, mut term) = make_proc_term(24, 80);
        // DA1 応答は 11 バイト ("\x1b[?62;4c")。MAX_PENDING_WRITES / 11 回以上送れば上限に達する。
        let repetitions = MAX_PENDING_WRITES / 11 + 100;
        for _ in 0..repetitions {
            proc.advance(&mut term, b"\x1b[c");
        }
        // バッファが MAX_PENDING_WRITES を超えていないこと。
        assert!(term.pending_writes.len() <= MAX_PENDING_WRITES);
        // 少なくとも何かは書き込まれていること。
        assert!(!term.pending_writes.is_empty());
    }
}
