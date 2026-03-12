//! Ring-buffer backed storage for grid rows.
//!
//! Scrolling is O(1): only the `zero` offset is updated; physical memory stays
//! in place.  The logical-to-physical index mapping is:
//!
//! ```text
//! physical = (zero + logical) % inner.len()
//! ```

use std::ops::{Index, IndexMut};

use crate::grid::cell::GridCell;
use crate::grid::row::Row;

/// When `inner.len() > visible_lines + MAX_CACHE_SIZE`, `truncate()` will
/// release the excess memory.
pub const MAX_CACHE_SIZE: usize = 1000;

/// Ring-buffer storage for [`Row<T>`].
///
/// `len` is the number of rows that are logically in use (visible lines +
/// scrollback history).  `inner.len()` may exceed `len` due to pre-allocated
/// cache space.
#[derive(Debug)]
pub struct Storage<T> {
    /// Physical row storage.
    pub(super) inner: Vec<Row<T>>,
    /// Index into `inner` that corresponds to logical row 0.
    pub(super) zero: usize,
    /// Number of visible (viewport) rows.
    pub(super) visible_lines: usize,
    /// Number of rows currently in logical use (visible + scrollback).
    pub(super) len: usize,
}

impl<T: GridCell> Storage<T> {
    /// Create a new storage with `visible_lines` rows, each `columns` columns wide.
    pub fn new(visible_lines: usize, columns: usize) -> Self {
        let inner = (0..visible_lines).map(|_| Row::new(columns)).collect::<Vec<_>>();
        Self { inner, zero: 0, visible_lines, len: visible_lines }
    }

    /// Number of rows currently in logical use.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` when no rows are in use (should never happen in practice).
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// 物理的に割り当て済みの行数（= ring buffer の実容量）。
    /// `len()` はこれを超えないが、`len > physical_len` になる場合は行が重複する。
    pub fn physical_len(&self) -> usize {
        self.inner.len()
    }

    /// Map a logical row index to a physical index inside `inner`.
    fn compute_index(&self, logical: usize) -> usize {
        (self.zero + logical) % self.inner.len()
    }

    /// Rotate the ring buffer by `count` positions (positive = forward).
    ///
    /// This is an O(1) operation; only `zero` is updated.
    pub fn rotate(&mut self, count: isize) {
        debug_assert!(!self.inner.is_empty(), "rotate on empty storage");
        let len = self.inner.len();
        // Safety: `len` is at most usize::MAX / 2 in any realistic terminal
        // grid (thousands of rows), so the cast to isize cannot wrap.
        #[allow(clippy::cast_possible_wrap)]
        let len_isize = len as isize;
        let shift = count.rem_euclid(len_isize) as usize;
        self.zero = (self.zero + shift) % len;
    }

    /// Swap two logical rows.
    pub fn swap(&mut self, a: usize, b: usize) {
        let pa = self.compute_index(a);
        let pb = self.compute_index(b);
        self.inner.swap(pa, pb);
    }

    /// Add `lines_added` visible rows at the bottom of the viewport.
    ///
    /// New rows are initialised with `columns` columns and `template` attributes.
    pub fn grow_visible(&mut self, lines_added: usize, columns: usize, template: &T) {
        // Ensure enough physical rows exist.
        self.initialize(lines_added, columns, template);
        self.visible_lines += lines_added;
        self.len += lines_added;
    }

    /// Remove `lines_removed` visible rows from the viewport.
    ///
    /// Excess rows become part of the scrollback cache; call `truncate()` to
    /// reclaim the memory if desired.
    pub fn shrink_visible(&mut self, lines_removed: usize) {
        let lines_removed = lines_removed.min(self.visible_lines);
        self.visible_lines -= lines_removed;
        self.len -= lines_removed;
    }

    /// Allocate `additional` new rows (appended to `inner`) with `columns`
    /// columns and `template` attributes.
    pub fn initialize(&mut self, additional: usize, columns: usize, template: &T) {
        // Reuse pre-allocated cache rows first if possible (they are beyond
        // `len` but within `inner`).
        let available = self.inner.len().saturating_sub(self.len);
        if additional <= available {
            // Re-initialise existing cache rows.
            for i in 0..additional {
                let idx = self.compute_index(self.len + i);
                self.inner[idx].reset(template);
                self.inner[idx].grow(columns);
            }
        } else {
            // Need to push fresh rows.
            let need_push = additional - available;
            // Re-initialise the cached ones.
            for i in 0..available {
                let idx = self.compute_index(self.len + i);
                self.inner[idx].reset(template);
                self.inner[idx].grow(columns);
            }
            for _ in 0..need_push {
                self.inner.push(Row::new(columns));
            }
        }
    }

    /// Release memory for rows beyond `visible_lines + MAX_CACHE_SIZE`.
    pub fn truncate(&mut self) {
        let max_len = self.visible_lines + MAX_CACHE_SIZE;
        if self.inner.len() <= max_len {
            return;
        }
        // Re-zero the buffer so physical indices are contiguous, then truncate.
        self.rezero();
        self.inner.truncate(self.len.min(max_len));
        if self.inner.len() < self.len {
            self.len = self.inner.len();
        }
    }

    /// Compact the ring buffer so that logical row 0 maps to physical index 0.
    fn rezero(&mut self) {
        if self.zero == 0 {
            return;
        }
        self.inner.rotate_left(self.zero);
        self.zero = 0;
    }
}

impl<T: GridCell> Index<usize> for Storage<T> {
    type Output = Row<T>;

    fn index(&self, logical: usize) -> &Row<T> {
        let physical = self.compute_index(logical);
        &self.inner[physical]
    }
}

impl<T: GridCell> IndexMut<usize> for Storage<T> {
    fn index_mut(&mut self, logical: usize) -> &mut Row<T> {
        let physical = self.compute_index(logical);
        &mut self.inner[physical]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::cell::Cell;
    use crate::index::Column;

    fn make_storage(lines: usize) -> Storage<Cell> {
        Storage::new(lines, 80)
    }

    #[test]
    fn new_storage_has_correct_len() {
        let s = make_storage(24);
        assert_eq!(s.len(), 24);
        assert_eq!(s.visible_lines, 24);
    }

    #[test]
    fn rotate_is_circular() {
        let mut s = make_storage(4);
        // Write a marker in logical row 0.
        s[0][Column(0)].c = 'X';
        // After rotating by 1, logical row 3 should contain the marker.
        s.rotate(1);
        assert_eq!(s[3][Column(0)].c, 'X');
    }

    #[test]
    fn rotate_negative() {
        let mut s = make_storage(4);
        s[1][Column(0)].c = 'Y';
        s.rotate(-1);
        assert_eq!(s[2][Column(0)].c, 'Y');
    }

    #[test]
    fn swap_two_rows() {
        let mut s = make_storage(4);
        s[0][Column(0)].c = 'A';
        s[2][Column(0)].c = 'B';
        s.swap(0, 2);
        assert_eq!(s[0][Column(0)].c, 'B');
        assert_eq!(s[2][Column(0)].c, 'A');
    }

    #[test]
    fn grow_visible_increases_len() {
        let mut s = make_storage(10);
        let template = Cell::default();
        s.grow_visible(5, 80, &template);
        assert_eq!(s.visible_lines, 15);
        assert_eq!(s.len(), 15);
    }

    #[test]
    fn shrink_visible_decreases_visible_lines() {
        let mut s = make_storage(10);
        s.shrink_visible(3);
        assert_eq!(s.visible_lines, 7);
        assert_eq!(s.len(), 7);
    }

    #[test]
    fn truncate_releases_excess() {
        let mut s = make_storage(5);
        let template = Cell::default();
        // Add many extra rows to exceed cache limit.
        s.grow_visible(MAX_CACHE_SIZE + 10, 80, &template);
        s.truncate();
        assert!(s.inner.len() <= s.visible_lines + MAX_CACHE_SIZE);
    }
}
