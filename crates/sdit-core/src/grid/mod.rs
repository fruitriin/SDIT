//! Terminal grid: cell storage, scrolling, and cursor management.
//!
//! The main type is [`Grid<T>`].  It wraps a ring-buffer [`Storage<T>`] and
//! adds cursor tracking, scrollback limit enforcement, and viewport scrolling.

pub mod cell;
pub mod row;
pub mod storage;

pub use cell::{Cell, CellFlags, Color, GridCell, NamedColor};
pub use row::Row;
pub use storage::Storage;

use std::ops::{Index, IndexMut, Range};

use crate::index::{Column, Line, Point};

// ---------------------------------------------------------------------------
// Scroll
// ---------------------------------------------------------------------------

/// Direction / amount to scroll the *display* (not the terminal buffer).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scroll {
    /// Move display offset by `delta` lines (positive = scroll up into history).
    Delta(isize),
    /// Jump to the very top of scrollback.
    Top,
    /// Jump back to the live viewport bottom.
    Bottom,
}

// ---------------------------------------------------------------------------
// Dimensions trait
// ---------------------------------------------------------------------------

/// Read-only size information for a grid.
pub trait Dimensions {
    /// Total number of logical rows (visible + scrollback history).
    fn total_lines(&self) -> usize;
    /// Number of visible rows (viewport height).
    fn screen_lines(&self) -> usize;
    /// Number of columns (viewport width).
    fn columns(&self) -> usize;
}

// ---------------------------------------------------------------------------
// Cursor
// ---------------------------------------------------------------------------

/// Cursor state stored inside the grid.
#[derive(Debug, Clone)]
pub struct Cursor<T: GridCell> {
    /// Current cursor position in viewport coordinates.
    pub point: Point,
    /// Attribute template; new cells are initialised from this.
    pub template: T,
    /// When `true`, the cursor is at the right margin and the next character
    /// should wrap to the next line before being placed.
    pub input_needs_wrap: bool,
}

impl<T: GridCell> Default for Cursor<T> {
    fn default() -> Self {
        Self {
            point: Point::new(Line(0), Column(0)),
            template: T::default(),
            input_needs_wrap: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Grid
// ---------------------------------------------------------------------------

/// The main grid data structure.
///
/// Rows are indexed 0 (top of viewport) .. `lines-1` (bottom).  Scrollback
/// rows are stored behind the viewport (negative logical indices relative to
/// the viewport top) but are accessed via [`display_offset`].
pub struct Grid<T: GridCell> {
    /// Ring-buffer row storage.
    raw: Storage<T>,
    /// Active cursor.
    pub cursor: Cursor<T>,
    /// Saved cursor (DECSC / DECRC).
    pub saved_cursor: Cursor<T>,
    /// Number of visible rows.
    lines: usize,
    /// Number of columns.
    columns: usize,
    /// How many lines the display is scrolled back into history.
    /// 0 = showing live viewport.
    display_offset: usize,
    /// Maximum number of scrollback lines to retain.
    max_scroll_limit: usize,
}

impl<T: GridCell> Grid<T> {
    /// Create a new grid with `lines` rows, `columns` columns, and the given
    /// scrollback limit.
    ///
    /// `lines` must be at least 1; zero is clamped to 1.
    pub fn new(lines: usize, columns: usize, max_scroll_limit: usize) -> Self {
        let lines = lines.max(1);
        Self {
            raw: Storage::new(lines, columns),
            cursor: Cursor::default(),
            saved_cursor: Cursor::default(),
            lines,
            columns,
            display_offset: 0,
            max_scroll_limit,
        }
    }

    /// Number of scrollback (history) lines currently stored.
    pub fn history_size(&self) -> usize {
        self.raw.len().saturating_sub(self.lines)
    }

    /// 現在のビューポートスクロールオフセットを返す。
    /// 0 = ライブビューポート。正値 = 履歴方向にスクロール中。
    pub fn display_offset(&self) -> usize {
        self.display_offset
    }

    /// Scroll the *terminal buffer* up by `count` lines within `region`.
    ///
    /// Lines scrolled above `region.start` are pushed into scrollback history
    /// (only when `region.start == 0`).
    pub fn scroll_up(&mut self, region: Range<usize>, count: usize) {
        if region.end <= region.start {
            return;
        }
        let count = count.min(region.end - region.start);

        if region.start == 0 {
            // Push lines into history.
            let scrollback_additions = count.min(self.max_scroll_limit);

            // Rotate so that the "oldest" lines end up behind the viewport.
            // Safety: `count` is bounded by the region length which is at most
            // `self.lines` — far below isize::MAX on any realistic platform.
            #[allow(clippy::cast_possible_wrap)]
            self.raw.rotate(count as isize);

            // Grow len to account for new history lines (up to limit).
            let grow =
                scrollback_additions.min(self.max_scroll_limit.saturating_sub(self.history_size()));
            if grow > 0 {
                // Extend logical len without adding new physical rows yet —
                // the rotation already brought old rows past the end; we just
                // account for them.
                self.raw.len += grow;
            }

            // Trim history that exceeds the limit.
            self.enforce_scroll_limit();
        } else {
            // Scroll within a sub-region (no history).
            for _ in 0..count {
                for i in region.start..region.end - 1 {
                    // Offset by history_size so we address viewport rows.
                    let hist = self.history_size();
                    self.raw.swap(hist + i, hist + i + 1);
                }
            }
        }

        // Reset the newly-exposed lines at the bottom of the region.
        let template = self.cursor.template.clone();
        let hist = self.history_size();
        for i in (region.end - count)..region.end {
            self.raw[hist + i].reset(&template);
        }
    }

    /// Scroll the *terminal buffer* down by `count` lines within `region`.
    ///
    /// Lines are moved down; lines that would fall off the bottom are
    /// discarded.
    pub fn scroll_down(&mut self, region: Range<usize>, count: usize) {
        if region.end <= region.start {
            return;
        }
        let count = count.min(region.end - region.start);
        let hist = self.history_size();

        // Move lines downward (from bottom to top to avoid overwriting).
        for _ in 0..count {
            for i in (region.start + 1..region.end).rev() {
                self.raw.swap(hist + i, hist + i - 1);
            }
        }

        // Clear the newly-exposed lines at the top of the region.
        let template = self.cursor.template.clone();
        for i in region.start..(region.start + count) {
            self.raw[hist + i].reset(&template);
        }
    }

    /// Update `display_offset` to scroll the *viewport* (not the buffer).
    pub fn scroll_display(&mut self, scroll: Scroll) {
        match scroll {
            Scroll::Delta(delta) => {
                let history = self.history_size();
                if delta > 0 {
                    let up = delta as usize;
                    self.display_offset = self.display_offset.saturating_add(up).min(history);
                } else {
                    let down = (-delta) as usize;
                    self.display_offset = self.display_offset.saturating_sub(down);
                }
            }
            Scroll::Top => {
                self.display_offset = self.history_size();
            }
            Scroll::Bottom => {
                self.display_offset = 0;
            }
        }
    }

    /// Resize the grid to `new_lines` rows and `new_columns` columns.
    ///
    /// Cursor position is clamped to stay within bounds.
    pub fn resize(&mut self, new_lines: usize, new_columns: usize) {
        let template = self.cursor.template.clone();

        // Grow or shrink columns.
        if new_columns > self.columns {
            let hist_len = self.raw.len();
            for i in 0..hist_len {
                self.raw[i].grow(new_columns);
            }
        } else if new_columns < self.columns {
            let hist_len = self.raw.len();
            for i in 0..hist_len {
                self.raw[i].shrink(new_columns);
            }
        }
        self.columns = new_columns;

        // Grow or shrink lines.
        if new_lines > self.lines {
            let add = new_lines - self.lines;
            self.raw.grow_visible(add, new_columns, &template);
        } else if new_lines < self.lines {
            let remove = self.lines - new_lines;
            self.raw.shrink_visible(remove);
        }
        self.lines = new_lines;

        // Clamp cursor.
        // Terminal grids are far smaller than i32::MAX rows in practice.
        let lines_i32 = i32::try_from(self.lines).unwrap_or(i32::MAX);
        let max_line = lines_i32.saturating_sub(1).max(0);
        let max_col = self.columns.saturating_sub(1);
        self.cursor.point.line = Line(self.cursor.point.line.0.min(max_line));
        self.cursor.point.column = Column(self.cursor.point.column.0.min(max_col));
        self.cursor.input_needs_wrap = false;

        // Clamp saved cursor as well.
        self.saved_cursor.point.line = Line(self.saved_cursor.point.line.0.min(max_line));
        self.saved_cursor.point.column = Column(self.saved_cursor.point.column.0.min(max_col));
        self.saved_cursor.input_needs_wrap = false;

        // Clamp display_offset.
        self.display_offset = self.display_offset.min(self.history_size());

        self.raw.truncate();
    }

    /// Clear all visible rows in the viewport.
    pub fn clear_viewport(&mut self) {
        let template = self.cursor.template.clone();
        let hist = self.history_size();
        for i in 0..self.lines {
            self.raw[hist + i].reset(&template);
        }
    }

    /// Discard all scrollback history, keeping only the visible viewport.
    pub fn clear_history(&mut self) {
        // Rezero so the viewport is at the front, then reset len.
        let hist = self.history_size();
        if hist == 0 {
            return;
        }
        // Rotate so the viewport's first row becomes physical index 0.
        // Safety: history size is bounded by max_scroll_limit which is far below isize::MAX.
        #[allow(clippy::cast_possible_wrap)]
        self.raw.rotate(hist as isize);
        self.raw.len = self.lines;
        self.display_offset = 0;
        self.raw.truncate();
    }

    /// Return a mutable reference to the cell at the current cursor position.
    pub fn cursor_cell(&mut self) -> &mut T {
        let line = self.cursor.point.line.as_viewport_idx();
        let col = self.cursor.point.column;
        let hist = self.history_size();
        &mut self.raw[hist + line][col]
    }

    /// raw インデックスで行・列を参照する。
    ///
    /// `raw_row` は `0..total_lines()` の範囲。`0` = 最古の履歴行、`history_size` = ビューポート先頭。
    /// `col` が範囲外の場合は `None` を返す。
    pub fn raw_row_cell(&self, raw_row: usize, col: usize) -> Option<&T> {
        if raw_row >= self.raw.len() || col >= self.columns {
            return None;
        }
        let row = &self.raw[raw_row];
        row.cells().get(col)
    }

    /// 全行を raw storage の順で参照するイテレータを返す。
    ///
    /// `0..history_size` が履歴行（最古 = 0）、`history_size..total_lines` がビューポート行。
    /// 返される行数は `min(total_lines, physical_capacity)` で、物理的に別個の行のみを返す。
    pub fn iter_raw_rows(&self) -> impl Iterator<Item = &Row<T>> {
        // raw.len() が物理行数を超える場合は重複が生じるため、物理行数で打ち切る。
        let count = self.raw.len().min(self.raw.physical_len());
        (0..count).map(move |i| &self.raw[i])
    }

    /// Enforce the scrollback line limit, discarding the oldest history rows.
    fn enforce_scroll_limit(&mut self) {
        let max_total = self.lines + self.max_scroll_limit;
        if self.raw.len > max_total {
            self.raw.len = max_total;
        }
    }
}

// ---------------------------------------------------------------------------
// Dimensions impl
// ---------------------------------------------------------------------------

impl<T: GridCell> Dimensions for Grid<T> {
    fn total_lines(&self) -> usize {
        self.raw.len()
    }

    fn screen_lines(&self) -> usize {
        self.lines
    }

    fn columns(&self) -> usize {
        self.columns
    }
}

// ---------------------------------------------------------------------------
// Index<Point> — viewport-relative coordinates
// ---------------------------------------------------------------------------

impl<T: GridCell> Index<Point> for Grid<T> {
    type Output = T;

    fn index(&self, point: Point) -> &T {
        let row = self.history_size() + point.line.as_viewport_idx();
        &self.raw[row][point.column]
    }
}

impl<T: GridCell> IndexMut<Point> for Grid<T> {
    fn index_mut(&mut self, point: Point) -> &mut T {
        let row = self.history_size() + point.line.as_viewport_idx();
        &mut self.raw[row][point.column]
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::{Column, Line, Point};

    type TestGrid = Grid<Cell>;

    fn make_grid(lines: usize, cols: usize) -> TestGrid {
        Grid::new(lines, cols, 100)
    }

    // --- Creation ---

    #[test]
    fn new_grid_dimensions() {
        let g = make_grid(24, 80);
        assert_eq!(g.screen_lines(), 24);
        assert_eq!(g.columns(), 80);
        assert_eq!(g.total_lines(), 24);
        assert_eq!(g.history_size(), 0);
    }

    // --- Cursor ---

    #[test]
    fn cursor_cell_default() {
        let mut g = make_grid(10, 40);
        let cell = g.cursor_cell();
        assert_eq!(cell.c, ' ');
    }

    #[test]
    fn cursor_cell_write() {
        let mut g = make_grid(10, 40);
        g.cursor.point = Point::new(Line(2), Column(5));
        g.cursor_cell().c = 'A';
        assert_eq!(g[Point::new(Line(2), Column(5))].c, 'A');
    }

    // --- Scroll up ---

    #[test]
    fn scroll_up_full_region_pushes_history() {
        let mut g = make_grid(5, 10);
        // Write a marker on the top line.
        g[Point::new(Line(0), Column(0))].c = 'H';
        g.scroll_up(0..5, 1);
        // History should have grown by 1.
        assert_eq!(g.history_size(), 1);
        // The top viewport line should be blank now.
        assert_eq!(g[Point::new(Line(0), Column(0))].c, ' ');
    }

    #[test]
    fn scroll_up_clears_bottom_lines() {
        let mut g = make_grid(5, 10);
        g[Point::new(Line(4), Column(0))].c = 'Z';
        g.scroll_up(0..5, 1);
        assert_eq!(g[Point::new(Line(4), Column(0))].c, ' ');
    }

    #[test]
    fn scroll_up_respects_max_limit() {
        let mut g: Grid<Cell> = Grid::new(5, 10, 3);
        for _ in 0..10 {
            g.scroll_up(0..5, 1);
        }
        assert!(g.history_size() <= 3);
    }

    // --- Scroll down ---

    #[test]
    fn scroll_down_moves_content() {
        let mut g = make_grid(5, 10);
        g[Point::new(Line(0), Column(0))].c = 'T';
        g.scroll_down(0..5, 1);
        // The marker should now be on line 1.
        assert_eq!(g[Point::new(Line(1), Column(0))].c, 'T');
        // Line 0 should be blank.
        assert_eq!(g[Point::new(Line(0), Column(0))].c, ' ');
    }

    // --- Display scroll ---

    #[test]
    fn scroll_display_delta() {
        let mut g = make_grid(5, 10);
        // Push two history lines.
        g.scroll_up(0..5, 2);
        g.scroll_display(Scroll::Delta(1));
        assert_eq!(g.display_offset, 1);
        g.scroll_display(Scroll::Bottom);
        assert_eq!(g.display_offset, 0);
        g.scroll_display(Scroll::Top);
        assert_eq!(g.display_offset, g.history_size());
    }

    #[test]
    fn scroll_display_clamped() {
        let mut g = make_grid(5, 10);
        g.scroll_display(Scroll::Delta(100));
        assert_eq!(g.display_offset, 0); // no history yet
    }

    // --- Resize ---

    #[test]
    fn resize_wider() {
        let mut g = make_grid(10, 40);
        g.resize(10, 80);
        assert_eq!(g.columns(), 80);
        assert_eq!(g.screen_lines(), 10);
    }

    #[test]
    fn resize_taller() {
        let mut g = make_grid(10, 40);
        g.resize(20, 40);
        assert_eq!(g.screen_lines(), 20);
        assert_eq!(g.total_lines(), 20);
    }

    #[test]
    fn resize_smaller() {
        let mut g = make_grid(20, 80);
        g.cursor.point = Point::new(Line(19), Column(79));
        g.resize(10, 40);
        assert_eq!(g.screen_lines(), 10);
        assert_eq!(g.columns(), 40);
        // Cursor must be clamped inside the new bounds.
        assert!(g.cursor.point.line.0 < 10);
        assert!(g.cursor.point.column.0 < 40);
    }

    // --- Clear ---

    #[test]
    fn clear_viewport_blanks_visible_area() {
        let mut g = make_grid(5, 10);
        g[Point::new(Line(2), Column(3))].c = 'X';
        g.clear_viewport();
        assert_eq!(g[Point::new(Line(2), Column(3))].c, ' ');
    }

    #[test]
    fn clear_history_removes_scrollback() {
        let mut g = make_grid(5, 10);
        g.scroll_up(0..5, 3);
        assert!(g.history_size() > 0);
        g.clear_history();
        assert_eq!(g.history_size(), 0);
        assert_eq!(g.display_offset, 0);
    }
}
