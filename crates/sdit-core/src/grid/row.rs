//! A single grid row backed by a `Vec<T>`.

use std::ops::{Index, IndexMut};

use crate::grid::cell::GridCell;
use crate::index::Column;

/// One row of terminal cells.
///
/// `occ` tracks the highest column that has been written to, which enables
/// efficient `is_clear()` checks and smarter serialisation in future.
#[derive(Debug, Clone)]
pub struct Row<T> {
    /// Cell storage (length == column count of the grid).
    pub(super) inner: Vec<T>,
    /// Occupation: one past the last column that was written to.
    pub(super) occ: usize,
}

impl<T: GridCell> Row<T> {
    /// Create a new row with `columns` default cells.
    pub fn new(columns: usize) -> Self {
        Self { inner: (0..columns).map(|_| T::default()).collect(), occ: 0 }
    }

    /// Grow the row to `columns` columns by appending default cells.
    pub fn grow(&mut self, columns: usize) {
        if columns > self.inner.len() {
            self.inner.resize_with(columns, T::default);
        }
    }

    /// Shrink the row to `columns` columns, returning the removed cells.
    ///
    /// If `columns >= self.len()` this is a no-op and returns an empty `Vec`.
    pub fn shrink(&mut self, columns: usize) -> Vec<T> {
        if columns >= self.inner.len() {
            return Vec::new();
        }
        let excess = self.inner.split_off(columns);
        self.occ = self.occ.min(columns);
        excess
    }

    /// Reset every cell in the row using `template` as the attribute source.
    pub fn reset(&mut self, template: &T) {
        for cell in &mut self.inner {
            cell.reset(template);
        }
        self.occ = 0;
    }

    /// Returns `true` when every cell in the row is empty (no visible content).
    pub fn is_clear(&self) -> bool {
        self.inner[..self.occ].iter().all(GridCell::is_empty)
    }

    /// Number of columns in this row.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Always `false`; a row always has at least one column.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// 全セルのスライスを返す。
    pub fn cells(&self) -> &[T] {
        &self.inner
    }
}

// --- Index by Column ---

impl<T: GridCell> Index<Column> for Row<T> {
    type Output = T;

    fn index(&self, col: Column) -> &T {
        &self.inner[col.0]
    }
}

impl<T: GridCell> IndexMut<Column> for Row<T> {
    fn index_mut(&mut self, col: Column) -> &mut T {
        // Track the highest written column for `occ`.
        if col.0 >= self.occ {
            self.occ = col.0 + 1;
        }
        &mut self.inner[col.0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::cell::Cell;

    #[test]
    fn new_row_is_clear() {
        let row: Row<Cell> = Row::new(80);
        assert_eq!(row.len(), 80);
        assert!(row.is_clear());
    }

    #[test]
    fn write_marks_occ() {
        let mut row: Row<Cell> = Row::new(80);
        row[Column(10)].c = 'A';
        assert_eq!(row.occ, 11);
    }

    #[test]
    fn is_clear_after_write() {
        let mut row: Row<Cell> = Row::new(80);
        row[Column(5)].c = 'X';
        assert!(!row.is_clear());
    }

    #[test]
    fn grow_extends_row() {
        let mut row: Row<Cell> = Row::new(40);
        row.grow(80);
        assert_eq!(row.len(), 80);
    }

    #[test]
    fn shrink_returns_excess() {
        let mut row: Row<Cell> = Row::new(80);
        row[Column(60)].c = 'Z';
        let excess = row.shrink(40);
        assert_eq!(row.len(), 40);
        // The written column was beyond the new width, so it ends up in excess.
        assert_eq!(excess.len(), 40);
    }

    #[test]
    fn shrink_noop_when_wider() {
        let mut row: Row<Cell> = Row::new(40);
        let excess = row.shrink(80);
        assert!(excess.is_empty());
        assert_eq!(row.len(), 40);
    }

    #[test]
    fn reset_clears_row() {
        let mut row: Row<Cell> = Row::new(80);
        row[Column(3)].c = 'Q';
        let template = Cell::default();
        row.reset(&template);
        assert!(row.is_clear());
        assert_eq!(row.occ, 0);
    }
}
