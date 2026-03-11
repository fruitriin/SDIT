//! Terminal grid coordinate types.
//!
//! - [`Line`]: row index (negative = scrollback history, 0+ = visible area)
//! - [`Column`]: column index
//! - [`Point`]: (line, column) pair

use std::fmt;
use std::ops::{Add, AddAssign, Sub, SubAssign};

/// Row index.
///
/// Negative values address scrollback history lines; non-negative values
/// address the visible viewport (0 = top of viewport).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Line(pub i32);

impl Line {
    /// Convert to a viewport row index (`usize`), clamping negative values to 0.
    ///
    /// Viewport coordinates are always non-negative.  This method provides a
    /// single, safe conversion point instead of scattering `.0.max(0) as usize`
    /// across the codebase.
    #[must_use]
    #[inline]
    pub fn as_viewport_idx(self) -> usize {
        self.0.max(0) as usize
    }
}

/// Column index (0-based).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Column(pub usize);

/// A 2-D point in the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Point {
    pub line: Line,
    pub column: Column,
}

impl Point {
    #[must_use]
    pub fn new(line: Line, column: Column) -> Self {
        Self { line, column }
    }
}

// --- Line arithmetic ---

impl Add<i32> for Line {
    type Output = Self;
    fn add(self, rhs: i32) -> Self {
        Self(self.0.saturating_add(rhs))
    }
}

impl AddAssign<i32> for Line {
    fn add_assign(&mut self, rhs: i32) {
        self.0 = self.0.saturating_add(rhs);
    }
}

impl Sub<i32> for Line {
    type Output = Self;
    fn sub(self, rhs: i32) -> Self {
        Self(self.0.saturating_sub(rhs))
    }
}

impl SubAssign<i32> for Line {
    fn sub_assign(&mut self, rhs: i32) {
        self.0 = self.0.saturating_sub(rhs);
    }
}

impl Sub for Line {
    type Output = i32;
    fn sub(self, rhs: Self) -> i32 {
        self.0 - rhs.0
    }
}

impl fmt::Display for Line {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// --- Column arithmetic ---

impl Add<usize> for Column {
    type Output = Self;
    fn add(self, rhs: usize) -> Self {
        Self(self.0.saturating_add(rhs))
    }
}

impl AddAssign<usize> for Column {
    fn add_assign(&mut self, rhs: usize) {
        self.0 = self.0.saturating_add(rhs);
    }
}

impl Sub<usize> for Column {
    type Output = Self;
    fn sub(self, rhs: usize) -> Self {
        Self(self.0.saturating_sub(rhs))
    }
}

impl SubAssign<usize> for Column {
    fn sub_assign(&mut self, rhs: usize) {
        self.0 = self.0.saturating_sub(rhs);
    }
}

impl Sub for Column {
    type Output = usize;
    fn sub(self, rhs: Self) -> usize {
        self.0.saturating_sub(rhs.0)
    }
}

impl fmt::Display for Column {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// --- From conversions ---

impl From<i32> for Line {
    fn from(v: i32) -> Self {
        Self(v)
    }
}

impl From<usize> for Line {
    fn from(v: usize) -> Self {
        // Line values originate from terminal row indices (< 65536 in practice).
        Self(i32::try_from(v).unwrap_or(i32::MAX))
    }
}

impl From<usize> for Column {
    fn from(v: usize) -> Self {
        Self(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_arithmetic() {
        let l = Line(5);
        assert_eq!((l + 3).0, 8);
        assert_eq!((l - 2).0, 3);
        assert_eq!(Line(10) - Line(3), 7);
    }

    #[test]
    fn line_saturating() {
        // i32::MAX + 1 saturates at i32::MAX.
        assert_eq!((Line(i32::MAX) + 1).0, i32::MAX);
        // i32::MIN - 1 saturates at i32::MIN.
        assert_eq!((Line(i32::MIN) - 1).0, i32::MIN);
    }

    #[test]
    fn line_as_viewport_idx() {
        assert_eq!(Line(0).as_viewport_idx(), 0);
        assert_eq!(Line(5).as_viewport_idx(), 5);
        // Negative values clamp to 0.
        assert_eq!(Line(-1).as_viewport_idx(), 0);
        assert_eq!(Line(i32::MIN).as_viewport_idx(), 0);
    }

    #[test]
    fn column_arithmetic() {
        let c = Column(5);
        assert_eq!((c + 3).0, 8);
        assert_eq!((c - 2).0, 3);
        assert_eq!(Column(10) - Column(3), 7);
    }

    #[test]
    fn column_saturating_sub() {
        assert_eq!((Column(0) - 5).0, 0);
    }

    #[test]
    fn point_ord() {
        let a = Point::new(Line(1), Column(0));
        let b = Point::new(Line(1), Column(5));
        let c = Point::new(Line(2), Column(0));
        assert!(a < b);
        assert!(b < c);
    }
}
