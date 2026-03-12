//! Terminal cell type and related color/flag definitions.

use std::sync::Arc;

use bitflags::bitflags;

// ---------------------------------------------------------------------------
// Color
// ---------------------------------------------------------------------------

/// Standard 8+8 named terminal colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NamedColor {
    // Normal colors (0–7)
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    // Bright colors (8–15)
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
    // Semantic aliases used by the terminal state machine
    Foreground,
    Background,
}

/// Terminal color value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color {
    /// One of the 16 standard named colors (or semantic fg/bg).
    Named(NamedColor),
    /// 256-color palette index.
    Indexed(u8),
    /// True-color (24-bit) RGB value.
    Rgb { r: u8, g: u8, b: u8 },
}

impl Default for Color {
    fn default() -> Self {
        Self::Named(NamedColor::Foreground)
    }
}

// ---------------------------------------------------------------------------
// CellFlags
// ---------------------------------------------------------------------------

bitflags! {
    /// Visual and semantic attributes for a terminal cell.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct CellFlags: u16 {
        /// Bold text.
        const BOLD            = 0b0000_0000_0001;
        /// Italic text.
        const ITALIC          = 0b0000_0000_0010;
        /// Single underline.
        const UNDERLINE       = 0b0000_0000_0100;
        /// Reverse video (swap fg/bg).
        const INVERSE         = 0b0000_0000_1000;
        /// Dim / faint text.
        const DIM             = 0b0000_0001_0000;
        /// Invisible (hidden) text.
        const HIDDEN          = 0b0000_0010_0000;
        /// Strikethrough.
        const STRIKEOUT       = 0b0000_0100_0000;
        /// Cell is the last column of a line that soft-wrapped.
        const WRAPLINE        = 0b0000_1000_0000;
        /// Cell holds the left half of a wide (double-width) character.
        const WIDE_CHAR       = 0b0001_0000_0000;
        /// Cell is the right-half placeholder of a wide character.
        const WIDE_CHAR_SPACER = 0b0010_0000_0000;
    }
}

// ---------------------------------------------------------------------------
// GridCell trait
// ---------------------------------------------------------------------------

/// Interface required by [`super::Row`] and [`super::Grid`] for generic cells.
pub trait GridCell: Default + Clone {
    /// Returns `true` when the cell contains no visible content and default
    /// attributes (i.e. it is indistinguishable from a freshly-reset cell).
    fn is_empty(&self) -> bool;

    /// Reset this cell to match `template`, preserving no content.
    fn reset(&mut self, template: &Self);

    /// Read-only access to the cell's attribute flags.
    fn flags(&self) -> CellFlags;

    /// Mutable access to the cell's attribute flags.
    fn flags_mut(&mut self) -> &mut CellFlags;
}

// ---------------------------------------------------------------------------
// Cell
// ---------------------------------------------------------------------------

/// A single terminal grid cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    /// The Unicode scalar value displayed in this cell.
    pub c: char,
    /// Foreground color.
    pub fg: Color,
    /// Background color.
    pub bg: Color,
    /// Text-attribute bit-flags.
    pub flags: CellFlags,
    /// OSC 8 ハイパーリンク URL。Arc により同一 URL を持つセル間で共有される。
    pub hyperlink: Option<Arc<str>>,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            c: ' ',
            fg: Color::Named(NamedColor::Foreground),
            bg: Color::Named(NamedColor::Background),
            flags: CellFlags::empty(),
            hyperlink: None,
        }
    }
}

impl GridCell for Cell {
    fn is_empty(&self) -> bool {
        let default = Self::default();
        self.c == default.c
            && self.fg == default.fg
            && self.bg == default.bg
            && self.flags == default.flags
            && self.hyperlink.is_none()
    }

    fn reset(&mut self, template: &Self) {
        self.c = ' ';
        self.fg = template.fg;
        self.bg = template.bg;
        // Preserve no content flags; wrapline is also cleared on reset.
        self.flags = CellFlags::empty();
        self.hyperlink = None;
    }

    fn flags(&self) -> CellFlags {
        self.flags
    }

    fn flags_mut(&mut self) -> &mut CellFlags {
        &mut self.flags
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_cell_is_empty() {
        assert!(Cell::default().is_empty());
    }

    #[test]
    fn non_default_cell_not_empty() {
        let c = Cell { c: 'A', ..Cell::default() };
        assert!(!c.is_empty());
    }

    #[test]
    fn reset_clears_content() {
        let mut c = Cell {
            c: 'Z',
            fg: Color::Named(NamedColor::Red),
            bg: Color::Named(NamedColor::Blue),
            flags: CellFlags::BOLD | CellFlags::ITALIC,
            hyperlink: None,
        };
        let template = Cell::default();
        c.reset(&template);
        assert!(c.is_empty());
    }

    #[test]
    fn cell_flags_bitwise() {
        let flags = CellFlags::BOLD | CellFlags::UNDERLINE;
        assert!(flags.contains(CellFlags::BOLD));
        assert!(flags.contains(CellFlags::UNDERLINE));
        assert!(!flags.contains(CellFlags::ITALIC));
    }

    #[test]
    fn color_default_is_named_foreground() {
        assert_eq!(Color::default(), Color::Named(NamedColor::Foreground));
    }

    #[test]
    fn hyperlink_cell_not_empty() {
        let c = Cell { hyperlink: Some(Arc::from("https://example.com")), ..Cell::default() };
        assert!(!c.is_empty());
    }

    #[test]
    fn reset_clears_hyperlink() {
        let mut c =
            Cell { c: 'A', hyperlink: Some(Arc::from("https://example.com")), ..Cell::default() };
        c.reset(&Cell::default());
        assert!(c.hyperlink.is_none());
        assert!(c.is_empty());
    }
}
