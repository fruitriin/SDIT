//! VTE parser integration and Terminal state machine.
//!
//! [`Terminal`] holds a [`Grid<Cell>`] and implements [`vte::Perform`] to
//! mutate it in response to escape sequences.  [`Processor`] wraps a
//! [`vte::Parser`] to feed raw bytes into a [`Terminal`].

pub mod handler;
pub mod search;
pub mod url_detector;

use std::ops::Range;
use std::sync::Arc;

use bitflags::bitflags;
use vte::Perform;

use crate::grid::{Cell, CellFlags, Color, Dimensions, Grid, GridCell, NamedColor};
use crate::index::{Column, Line, Point};

/// `pending_writes` バッファの最大サイズ（バイト）。
/// 悪意あるプログラムが大量の DA/DSR/CPR リクエストを送信してメモリを枯渇させる
/// ことを防ぐ。超過分は破棄される。
const MAX_PENDING_WRITES: usize = 4096;

/// セマンティックマーカーの最大保持数。
/// 超過した場合は古いマーカーから削除する（メモリ安全性）。
const MAX_SEMANTIC_MARKERS: usize = 10_000;

// ---------------------------------------------------------------------------
// ShellIntegration — OSC 133 セマンティックゾーン
// ---------------------------------------------------------------------------

/// シェルインテグレーション: セマンティックゾーンの種類（OSC 133 / FinalTerm）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticZone {
    /// プロンプト開始 (OSC 133;A)
    PromptStart,
    /// コマンド入力開始（プロンプト終了）(OSC 133;B)
    CommandStart,
    /// コマンド出力開始 (OSC 133;C)
    OutputStart,
    /// コマンド終了 + 終了コード (OSC 133;D)
    CommandEnd(Option<i32>),
}

/// セマンティックマーカー（行番号 + ゾーン種別）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SemanticMarker {
    /// マーカーが記録された行（viewport 内の行番号として Grid の raw index）。
    pub line: i32,
    /// ゾーン種別。
    pub zone: SemanticZone,
}

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
// Kitty Keyboard Protocol
// ---------------------------------------------------------------------------

/// Kitty keyboard protocol のプログレッシブエンハンスメントフラグ。
///
/// 参照: <https://sw.kovidgoyal.net/kitty/keyboard-protocol/>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KittyKeyboardFlags(u8);

impl KittyKeyboardFlags {
    /// フラグなし（レガシー動作）。
    pub const NONE: Self = Self(0);
    /// Bit 0: エスケープコードの曖昧性を解消する。
    pub const DISAMBIGUATE: u8 = 1;
    /// Bit 1: イベントタイプ（press/repeat/release）を報告する。
    pub const REPORT_EVENTS: u8 = 2;
    /// Bit 2: 代替キーを報告する。
    pub const REPORT_ALTERNATES: u8 = 4;
    /// Bit 3: すべてのキーをエスケープコードとして報告する。
    pub const REPORT_ALL: u8 = 8;
    /// Bit 4: 関連テキストを報告する。
    pub const REPORT_ASSOCIATED: u8 = 16;

    /// 生の `u8` 値から生成する（上位ビットはマスクされる）。
    pub fn from_raw(val: u8) -> Self {
        Self(val & 0x1f)
    }

    /// 生の `u8` 値を返す。
    pub fn raw(self) -> u8 {
        self.0
    }

    /// 指定したフラグビットが立っているか返す。
    pub fn has(self, flag: u8) -> bool {
        self.0 & flag != 0
    }

    /// いずれかのフラグが有効かどうか返す。
    pub fn is_active(self) -> bool {
        self.0 != 0
    }
}

/// Kitty keyboard flags のプッシュ/ポップスタック。
///
/// Ghostty と同じく8エントリの固定サイズスタック。
/// オーバーフロー時はプッシュを無視する（サイレント失敗）。
#[derive(Debug, Clone)]
pub struct KittyFlagStack {
    entries: [KittyKeyboardFlags; 8],
    len: usize,
}

impl Default for KittyFlagStack {
    fn default() -> Self {
        Self {
            entries: [KittyKeyboardFlags::NONE; 8],
            len: 1, // 初期エントリ（NONE）を含む
        }
    }
}

impl KittyFlagStack {
    /// フラグをスタックにプッシュする。
    ///
    /// スタックが満杯（8エントリ）の場合はプッシュを無視する。
    pub fn push(&mut self, flags: KittyKeyboardFlags) {
        if self.len < 8 {
            self.entries[self.len] = flags;
            self.len += 1;
        } else {
            log::debug!("Kitty flag stack full, ignoring push");
        }
    }

    /// スタックの先頭から `n` エントリをポップする。
    ///
    /// 最低1エントリ（初期エントリ）は常に残る。
    pub fn pop(&mut self, n: usize) {
        let new_len = self.len.saturating_sub(n).max(1);
        for i in new_len..self.len {
            self.entries[i] = KittyKeyboardFlags::NONE;
        }
        self.len = new_len;
    }

    /// 現在のフラグ（スタックトップ）を返す。
    pub fn current(&self) -> KittyKeyboardFlags {
        self.entries[self.len - 1]
    }

    /// 現在のフラグ（スタックトップ）を上書きする。
    pub fn set(&mut self, flags: KittyKeyboardFlags) {
        self.entries[self.len - 1] = flags;
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
    /// カーソルスタイル（アプリケーションが DECSCUSR で変更した値）。
    pub(super) cursor_style: CursorStyle,
    /// カーソル点滅が有効か（アプリケーションが DECSCUSR で変更した値）。
    pub(super) cursor_blinking: bool,
    /// デフォルトカーソルスタイル（DECSCUSR 0 で復帰する基準値）。
    pub(super) default_cursor_style: CursorStyle,
    /// デフォルトカーソル点滅（DECSCUSR 0 で復帰する基準値）。
    pub(super) default_cursor_blinking: bool,
    /// BEL (0x07) を受信したか。
    pub(super) bell_pending: bool,
    /// OSC 52 クリップボード書き込み要求。
    pub(super) clipboard_write_pending: Option<String>,
    /// OSC 8 ハイパーリンク: 現在アクティブな URL。None はリンクなし。
    pub(super) current_hyperlink: Option<Arc<str>>,
    /// Kitty keyboard protocol フラグスタック。
    pub kitty_flags: KittyFlagStack,
    /// デスクトップ通知ペンディング（title, body）。OSC 9/99 で設定される。
    pub(super) notification_pending: Option<(String, String)>,
    /// セマンティックマーカーのリスト（時系列順）。OSC 133 で記録される。
    pub semantic_markers: Vec<SemanticMarker>,
    /// OSC 133 シェルインテグレーションを有効にするかどうか。
    ///
    /// Config への参照を持たないため、GUI 側（app.rs）で設定して渡す。
    /// デフォルト: `true`。
    pub shell_integration_enabled: bool,
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
            default_cursor_style: CursorStyle::default(),
            default_cursor_blinking: false,
            bell_pending: false,
            clipboard_write_pending: None,
            current_hyperlink: None,
            kitty_flags: KittyFlagStack::default(),
            notification_pending: None,
            semantic_markers: Vec::new(),
            shell_integration_enabled: true,
        }
    }

    /// Create a new [`Terminal`] with specified default cursor style and blinking.
    ///
    /// Both the initial cursor style/blinking and the "default" values (used
    /// when DECSCUSR 0 is received) are set to the given parameters.
    pub fn new_with_cursor(
        lines: usize,
        columns: usize,
        max_scroll_limit: usize,
        default_cursor_style: CursorStyle,
        default_cursor_blinking: bool,
    ) -> Self {
        let tabs = build_tabs(columns);
        Self {
            grid: Grid::new(lines, columns, max_scroll_limit),
            inactive_grid: Grid::new(lines, columns, 0),
            mode: TermMode::defaults(),
            scroll_region: 0..lines,
            tabs,
            title: None,
            pending_writes: Vec::new(),
            cursor_style: default_cursor_style,
            cursor_blinking: default_cursor_blinking,
            default_cursor_style,
            default_cursor_blinking,
            bell_pending: false,
            clipboard_write_pending: None,
            current_hyperlink: None,
            kitty_flags: KittyFlagStack::default(),
            notification_pending: None,
            semantic_markers: Vec::new(),
            shell_integration_enabled: true,
        }
    }

    /// デフォルトカーソルスタイルと点滅設定を更新する。
    ///
    /// 設定ファイルの変更（hot reload）で既存 Terminal に反映するために使用する。
    /// アプリケーションが DECSCUSR 0 を発行したとき、この値が使われる。
    /// 現在のカーソルスタイル・点滅は変更しない（アプリケーションによる変更を維持する）。
    pub fn set_default_cursor(&mut self, style: CursorStyle, blinking: bool) {
        self.default_cursor_style = style;
        self.default_cursor_blinking = blinking;
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

    /// デスクトップ通知ペンディングを取り出す。
    ///
    /// OSC 9 / OSC 99 で設定された `(title, body)` を返す。
    /// 呼び出し後はフィールドが `None` になる（take セマンティクス）。
    pub fn take_notification(&mut self) -> Option<(String, String)> {
        self.notification_pending.take()
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
        // 画面切替時にアクティブなハイパーリンクをクリア（意図しないリンク付与を防止）
        self.current_hyperlink = None;

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

    /// 現在のカーソル行から上方向に最も近い PromptStart マーカーを見つけて、その行番号を返す。
    pub fn prev_prompt(&self) -> Option<i32> {
        let current_line = self.grid.cursor.point.line.0;
        self.semantic_markers
            .iter()
            .rev()
            .filter(|m| m.zone == SemanticZone::PromptStart && m.line < current_line)
            .map(|m| m.line)
            .next()
    }

    /// 現在のカーソル行から下方向に最も近い PromptStart マーカーを見つけて、その行番号を返す。
    pub fn next_prompt(&self) -> Option<i32> {
        let current_line = self.grid.cursor.point.line.0;
        self.semantic_markers
            .iter()
            .filter(|m| m.zone == SemanticZone::PromptStart && m.line > current_line)
            .map(|m| m.line)
            .next()
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
            let hyperlink = self.current_hyperlink.clone();
            let cell = self.grid.cursor_cell();
            cell.c = c;
            cell.fg = tmpl.fg;
            cell.bg = tmpl.bg;
            cell.flags = tmpl.flags;
            cell.hyperlink = hyperlink;
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
        // OSC 133: シェルインテグレーション（FinalTerm セマンティックゾーン）。
        if self.shell_integration_enabled && params.len() >= 2 && params[0] == b"133" {
            let sub = params[1];
            let zone = if sub == b"A" {
                Some(SemanticZone::PromptStart)
            } else if sub == b"B" {
                Some(SemanticZone::CommandStart)
            } else if sub == b"C" {
                Some(SemanticZone::OutputStart)
            } else if sub == b"D" {
                // D の後に ;exit_code が続く場合がある: params[2] に exit code
                let exit_code = if params.len() >= 3 {
                    std::str::from_utf8(params[2]).ok().and_then(|s| s.parse::<i32>().ok())
                } else {
                    None
                };
                Some(SemanticZone::CommandEnd(exit_code))
            } else {
                None
            };
            if let Some(z) = zone {
                let line = self.grid.cursor.point.line.0;
                self.semantic_markers.push(SemanticMarker { line, zone: z });
                if self.semantic_markers.len() > MAX_SEMANTIC_MARKERS {
                    self.semantic_markers.remove(0);
                }
            }
            return;
        }

        // OSC 8: ハイパーリンク。
        // リンク開始: \e]8;params;URL\ST  (URL は http:// または https:// のみ)
        // リンク終了: \e]8;;\ST
        if params.len() >= 2 && params[0] == b"8" {
            // URL は params[2]（params が3要素以上の場合）。
            // params が2要素または params[2] が空なら終了。
            const MAX_URL_BYTES: usize = 2048;
            let url_bytes = if params.len() >= 3 { params[2] } else { b"" };
            if url_bytes.is_empty() {
                self.current_hyperlink = None;
            } else if url_bytes.len() <= MAX_URL_BYTES {
                if let Ok(url) = std::str::from_utf8(url_bytes) {
                    // http:// または https:// のみ受け付ける
                    if url.starts_with("http://") || url.starts_with("https://") {
                        self.current_hyperlink = Some(Arc::from(url));
                    }
                    // それ以外のスキーム（file:// 等）は無視（current_hyperlink は変更しない）
                }
            }
            return;
        }

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
        // OSC 9: iTerm2 互換デスクトップ通知。
        // 形式: \e]9;<body>BEL
        if params.len() >= 2 && params[0] == b"9" {
            const MAX_NOTIFY_BYTES: usize = 4096;
            let raw = params[1];
            let body = sanitize_notification_text(truncate_utf8(raw, MAX_NOTIFY_BYTES));
            self.notification_pending = Some(("SDIT".to_string(), body));
        }

        // OSC 99: Kitty 互換デスクトップ通知。
        // 形式: \e]99;<body>BEL （簡易実装: 本文のみ）
        if params.len() >= 2 && params[0] == b"99" {
            const MAX_NOTIFY_BYTES: usize = 4096;
            let raw = params[1];
            let body = sanitize_notification_text(truncate_utf8(raw, MAX_NOTIFY_BYTES));
            self.notification_pending = Some(("SDIT".to_string(), body));
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

/// 通知テキストから制御文字を除去する（改行・タブは保持）。
fn sanitize_notification_text(s: &str) -> String {
    s.chars().filter(|c| !c.is_control() || *c == '\n' || *c == '\t').collect()
}

/// バイト列を最大 `max_bytes` バイトで切り詰め、UTF-8 境界に合わせて返す。
///
/// マルチバイト文字の途中で切れた場合は、有効な部分までを返す。
fn truncate_utf8(s: &[u8], max_bytes: usize) -> &str {
    let capped = if s.len() > max_bytes { &s[..max_bytes] } else { s };
    match std::str::from_utf8(capped) {
        Ok(s) => s,
        Err(e) => std::str::from_utf8(&capped[..e.valid_up_to()]).unwrap_or(""),
    }
}

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
mod tests;
