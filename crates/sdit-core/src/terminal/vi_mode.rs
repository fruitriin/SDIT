//! vi モード（コピーモード）のカーソル移動ロジック。
//!
//! [`ViCursor`] は Grid に対してキーボードモーションを適用する。
//! 座標系は Grid の viewport 相対行インデックス（`Line`）を使用する。
//! - `Line(-history)` .. `Line(screen_lines - 1)` の全スクロールバック行をカバー。
//! - セルアクセスは `grid_cell_at` ヘルパーで行い、履歴行と viewport 行を統一的に扱う。

use crate::grid::{Cell, Dimensions, Grid};
use crate::index::{Column, Line, Point};

// ---------------------------------------------------------------------------
// ViMotion
// ---------------------------------------------------------------------------

/// vi モードのカーソル移動操作。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViMotion {
    /// k — 上に1行
    Up,
    /// j — 下に1行
    Down,
    /// h — 左に1列
    Left,
    /// l — 右に1列
    Right,
    /// 0 — 行頭
    First,
    /// $ — 行末
    Last,
    /// w — 次の単語先頭へ
    WordRight,
    /// b — 前の単語先頭へ
    WordLeft,
    /// e — 単語末尾へ
    WordEnd,
    /// { — 前の段落（空行）へ
    ParagraphUp,
    /// } — 次の段落（空行）へ
    ParagraphDown,
    /// gg — 最上行（スクロールバック先頭）へ
    Top,
    /// G — 最下行（ビューポート末尾）へ
    Bottom,
    /// H — 現在表示画面の先頭行へ
    ScreenTop,
    /// M — 現在表示画面の中央行へ
    ScreenMiddle,
    /// L — 現在表示画面の末尾行へ
    ScreenBottom,
}

// ---------------------------------------------------------------------------
// ViCursor
// ---------------------------------------------------------------------------

/// vi モードのカーソル。Grid 上の任意の行・列を指す。
///
/// `point.line` は viewport 相対行インデックス:
/// - `0` = ビューポート先頭行
/// - `screen_lines - 1` = ビューポート末尾行
/// - 負値 = スクロールバック履歴行（`-history_size` が最古行）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViCursor {
    /// 現在のカーソル位置。
    pub point: Point,
}

impl ViCursor {
    /// 新しい `ViCursor` を作成する。
    pub fn new(point: Point) -> Self {
        Self { point }
    }

    /// 指定したモーションを適用し、新しい `ViCursor` を返す。
    #[must_use]
    pub fn motion(mut self, grid: &Grid<Cell>, motion: ViMotion) -> Self {
        let screen_lines = grid.screen_lines();
        let columns = grid.columns();
        let history = grid.history_size();
        let display_offset = grid.display_offset();

        match motion {
            ViMotion::Up => {
                #[allow(clippy::cast_possible_wrap)]
                let min_line = -(history as i32);
                if self.point.line.0 > min_line {
                    self.point.line = Line(self.point.line.0 - 1);
                }
            }
            ViMotion::Down => {
                #[allow(clippy::cast_possible_wrap)]
                let max_line = (screen_lines as i32) - 1;
                if self.point.line.0 < max_line {
                    self.point.line = Line(self.point.line.0 + 1);
                }
            }
            ViMotion::Left => {
                self.point.column = Column(self.point.column.0.saturating_sub(1));
            }
            ViMotion::Right => {
                let max_col = columns.saturating_sub(1);
                if self.point.column.0 < max_col {
                    self.point.column = Column(self.point.column.0 + 1);
                }
            }
            ViMotion::First => {
                self.point.column = Column(0);
            }
            ViMotion::Last => {
                self.point.column = Column(columns.saturating_sub(1));
            }
            ViMotion::WordRight => {
                self.point = word_right(grid, self.point);
            }
            ViMotion::WordLeft => {
                self.point = word_left(grid, self.point);
            }
            ViMotion::WordEnd => {
                self.point = word_end(grid, self.point);
            }
            ViMotion::ParagraphUp => {
                self.point = paragraph_up(grid, self.point);
            }
            ViMotion::ParagraphDown => {
                self.point = paragraph_down(grid, self.point);
            }
            ViMotion::Top => {
                #[allow(clippy::cast_possible_wrap)]
                {
                    self.point = Point::new(Line(-(history as i32)), Column(0));
                }
            }
            ViMotion::Bottom => {
                #[allow(clippy::cast_possible_wrap)]
                {
                    self.point = Point::new(Line((screen_lines as i32) - 1), Column(0));
                }
            }
            ViMotion::ScreenTop => {
                #[allow(clippy::cast_possible_wrap)]
                {
                    let top_line = -(display_offset as i32);
                    self.point = Point::new(Line(top_line), Column(0));
                }
            }
            ViMotion::ScreenMiddle => {
                #[allow(clippy::cast_possible_wrap)]
                {
                    let top_line = -(display_offset as i32);
                    let mid = top_line + (screen_lines as i32) / 2;
                    self.point = Point::new(Line(mid), Column(0));
                }
            }
            ViMotion::ScreenBottom => {
                #[allow(clippy::cast_possible_wrap)]
                {
                    let top_line = -(display_offset as i32);
                    let bot = top_line + (screen_lines as i32) - 1;
                    self.point = Point::new(Line(bot), Column(0));
                }
            }
        }
        self
    }
}

// ---------------------------------------------------------------------------
// セルアクセスヘルパー
// ---------------------------------------------------------------------------

/// Grid の任意の行・列のセルを取得する。
///
/// `point.line` は viewport 相対インデックス（負値 = 履歴行）。
/// 範囲外の場合は空セル（スペース文字）を返す。
fn grid_cell_at(grid: &Grid<Cell>, point: Point) -> Cell {
    let screen_lines = grid.screen_lines();
    let columns = grid.columns();
    let history = grid.history_size();

    // 行の境界チェック
    #[allow(clippy::cast_possible_wrap)]
    let min_line = -(history as i32);
    #[allow(clippy::cast_possible_wrap)]
    let max_line = (screen_lines as i32) - 1;
    if point.line.0 < min_line || point.line.0 > max_line {
        return Cell::default();
    }

    // 列の境界チェック
    if point.column.0 >= columns {
        return Cell::default();
    }

    // viewport 相対インデックスに変換して raw アクセス
    // history_size + viewport_row がビューポート先頭の raw インデックス。
    // Line(0) = ビューポート先頭 (raw index = history_size)
    // Line(-1) = 1行前の履歴 (raw index = history_size - 1)
    let viewport_offset = point.line.0;
    #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
    let raw_idx = if viewport_offset >= 0 {
        history + viewport_offset as usize
    } else {
        let back = (-viewport_offset) as usize;
        if back > history {
            return Cell::default();
        }
        history - back
    };

    grid.raw_row_cell(raw_idx, point.column.0).cloned().unwrap_or_default()
}

/// セルが空白かどうかを判定する（スペース・タブ・null = 空セル）。
fn is_space(cell: &Cell) -> bool {
    matches!(cell.c, ' ' | '\t' | '\0')
}

/// Grid の行が空行（全セルが空白）かどうかを判定する。
fn is_empty_line(grid: &Grid<Cell>, line: Line) -> bool {
    let columns = grid.columns();
    for col in 0..columns {
        let cell = grid_cell_at(grid, Point::new(line, Column(col)));
        if !is_space(&cell) {
            return false;
        }
    }
    true
}

// ---------------------------------------------------------------------------
// 単語移動ヘルパー
// ---------------------------------------------------------------------------

/// w — 次の単語先頭へ移動する。
///
/// 非空白をスキップ → 空白をスキップ → 次の非空白の先頭に止まる。
fn word_right(grid: &Grid<Cell>, mut point: Point) -> Point {
    let screen_lines = grid.screen_lines();
    let columns = grid.columns();
    let history = grid.history_size();

    #[allow(clippy::cast_possible_wrap)]
    let max_line = (screen_lines as i32) - 1;
    #[allow(clippy::cast_possible_wrap)]
    let min_line = -(history as i32);

    // 1. 現在位置の非空白をスキップ
    loop {
        let cell = grid_cell_at(grid, point);
        if is_space(&cell) {
            break;
        }
        if point.column.0 + 1 >= columns {
            // 行末: 次の行へ
            if point.line.0 >= max_line {
                return point;
            }
            point.line = Line(point.line.0 + 1);
            point.column = Column(0);
        } else {
            point.column = Column(point.column.0 + 1);
        }
    }

    // 2. 空白をスキップ
    loop {
        let cell = grid_cell_at(grid, point);
        if !is_space(&cell) {
            break;
        }
        if point.column.0 + 1 >= columns {
            if point.line.0 >= max_line {
                return point;
            }
            point.line = Line(point.line.0 + 1);
            point.column = Column(0);
        } else {
            point.column = Column(point.column.0 + 1);
        }
        // 行全体が空なら次へ
        if point.column.0 == 0 && point.line.0 < max_line {
            // 行頭に来たばかり: そのまま継続
        }
        let _ = min_line; // suppress warning
    }

    point
}

/// b — 前の単語先頭へ移動する。
///
/// 左方向に空白をスキップ → 非空白をスキップ → 非空白の先頭に止まる。
fn word_left(grid: &Grid<Cell>, mut point: Point) -> Point {
    let history = grid.history_size();

    #[allow(clippy::cast_possible_wrap)]
    let min_line = -(history as i32);

    // 1. まず1つ左に進む
    if point.column.0 == 0 {
        if point.line.0 <= min_line {
            return point;
        }
        point.line = Line(point.line.0 - 1);
        let cols = grid.columns();
        point.column = Column(cols.saturating_sub(1));
    } else {
        point.column = Column(point.column.0 - 1);
    }

    // 2. 空白をスキップ（左方向）
    loop {
        let cell = grid_cell_at(grid, point);
        if !is_space(&cell) {
            break;
        }
        if point.column.0 == 0 {
            if point.line.0 <= min_line {
                return point;
            }
            point.line = Line(point.line.0 - 1);
            let cols = grid.columns();
            point.column = Column(cols.saturating_sub(1));
        } else {
            point.column = Column(point.column.0 - 1);
        }
    }

    // 3. 非空白をスキップ（左方向）
    loop {
        if point.column.0 == 0 {
            break; // 行頭に到達
        }
        let prev = Point::new(point.line, Column(point.column.0 - 1));
        let cell = grid_cell_at(grid, prev);
        if is_space(&cell) {
            break;
        }
        point.column = Column(point.column.0 - 1);
    }

    point
}

/// e — 単語末尾へ移動する。
///
/// 1つ右に進んでから空白をスキップ → 非空白の末尾に止まる。
fn word_end(grid: &Grid<Cell>, mut point: Point) -> Point {
    let screen_lines = grid.screen_lines();
    let columns = grid.columns();

    #[allow(clippy::cast_possible_wrap)]
    let max_line = (screen_lines as i32) - 1;

    // 1. 1つ右に進む
    if point.column.0 + 1 >= columns {
        if point.line.0 >= max_line {
            return point;
        }
        point.line = Line(point.line.0 + 1);
        point.column = Column(0);
    } else {
        point.column = Column(point.column.0 + 1);
    }

    // 2. 空白をスキップ
    loop {
        let cell = grid_cell_at(grid, point);
        if !is_space(&cell) {
            break;
        }
        if point.column.0 + 1 >= columns {
            if point.line.0 >= max_line {
                return point;
            }
            point.line = Line(point.line.0 + 1);
            point.column = Column(0);
        } else {
            point.column = Column(point.column.0 + 1);
        }
    }

    // 3. 非空白の末尾を探す（右方向）
    loop {
        if point.column.0 + 1 >= columns {
            break;
        }
        let next = Point::new(point.line, Column(point.column.0 + 1));
        let cell = grid_cell_at(grid, next);
        if is_space(&cell) {
            break;
        }
        point.column = Column(point.column.0 + 1);
    }

    point
}

// ---------------------------------------------------------------------------
// 段落移動ヘルパー
// ---------------------------------------------------------------------------

/// { — 前の段落（空行）へ移動する。
///
/// 上方向に非空行をスキップ → 空行をスキップしてその行に止まる。
fn paragraph_up(grid: &Grid<Cell>, mut point: Point) -> Point {
    let history = grid.history_size();

    #[allow(clippy::cast_possible_wrap)]
    let min_line = -(history as i32);

    // 1. 現在行が空行なら、まず非空行まで上へ
    while point.line.0 > min_line && is_empty_line(grid, point.line) {
        point.line = Line(point.line.0 - 1);
    }

    // 2. 非空行をスキップ（上方向）
    while point.line.0 > min_line && !is_empty_line(grid, point.line) {
        point.line = Line(point.line.0 - 1);
    }

    point.column = Column(0);
    point
}

/// } — 次の段落（空行）へ移動する。
///
/// 下方向に非空行をスキップ → 空行をスキップしてその行に止まる。
fn paragraph_down(grid: &Grid<Cell>, mut point: Point) -> Point {
    let screen_lines = grid.screen_lines();

    #[allow(clippy::cast_possible_wrap)]
    let max_line = (screen_lines as i32) - 1;

    // 1. 現在行が空行なら、まず非空行まで下へ
    while point.line.0 < max_line && is_empty_line(grid, point.line) {
        point.line = Line(point.line.0 + 1);
    }

    // 2. 非空行をスキップ（下方向）
    while point.line.0 < max_line && !is_empty_line(grid, point.line) {
        point.line = Line(point.line.0 + 1);
    }

    point.column = Column(0);
    point
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::Grid;

    fn make_grid(lines: usize, cols: usize) -> Grid<Cell> {
        Grid::new(lines, cols, 100)
    }

    fn write_char(grid: &mut Grid<Cell>, row: usize, col: usize, c: char) {
        #[allow(clippy::cast_possible_wrap)]
        let point = Point::new(Line(row as i32), Column(col));
        grid[point].c = c;
    }

    // --- 基本移動 ---

    #[test]
    fn vi_cursor_move_down() {
        let grid = make_grid(10, 80);
        let cursor = ViCursor::new(Point::new(Line(0), Column(0)));
        let moved = cursor.motion(&grid, ViMotion::Down);
        assert_eq!(moved.point.line.0, 1);
    }

    #[test]
    fn vi_cursor_move_up() {
        let grid = make_grid(10, 80);
        let cursor = ViCursor::new(Point::new(Line(5), Column(0)));
        let moved = cursor.motion(&grid, ViMotion::Up);
        assert_eq!(moved.point.line.0, 4);
    }

    #[test]
    fn vi_cursor_move_left() {
        let grid = make_grid(10, 80);
        let cursor = ViCursor::new(Point::new(Line(0), Column(5)));
        let moved = cursor.motion(&grid, ViMotion::Left);
        assert_eq!(moved.point.column.0, 4);
    }

    #[test]
    fn vi_cursor_move_right() {
        let grid = make_grid(10, 80);
        let cursor = ViCursor::new(Point::new(Line(0), Column(5)));
        let moved = cursor.motion(&grid, ViMotion::Right);
        assert_eq!(moved.point.column.0, 6);
    }

    #[test]
    fn vi_cursor_clamp_at_bottom() {
        let grid = make_grid(10, 80);
        let cursor = ViCursor::new(Point::new(Line(9), Column(0)));
        let moved = cursor.motion(&grid, ViMotion::Down);
        assert_eq!(moved.point.line.0, 9); // クランプ
    }

    #[test]
    fn vi_cursor_clamp_at_top_no_history() {
        let grid = make_grid(10, 80);
        let cursor = ViCursor::new(Point::new(Line(0), Column(0)));
        let moved = cursor.motion(&grid, ViMotion::Up);
        assert_eq!(moved.point.line.0, 0); // 履歴なし → クランプ
    }

    #[test]
    fn vi_cursor_top_motion() {
        let grid = make_grid(10, 80);
        let cursor = ViCursor::new(Point::new(Line(5), Column(10)));
        let moved = cursor.motion(&grid, ViMotion::Top);
        assert_eq!(moved.point.line.0, 0); // 履歴なし → Line(0)
        assert_eq!(moved.point.column.0, 0);
    }

    #[test]
    fn vi_cursor_bottom_motion() {
        let grid = make_grid(10, 80);
        let cursor = ViCursor::new(Point::new(Line(0), Column(0)));
        let moved = cursor.motion(&grid, ViMotion::Bottom);
        assert_eq!(moved.point.line.0, 9);
        assert_eq!(moved.point.column.0, 0);
    }

    #[test]
    fn vi_cursor_word_right_basic() {
        let mut grid = make_grid(5, 20);
        // "hello world" と書き込む
        for (i, c) in "hello".chars().enumerate() {
            write_char(&mut grid, 0, i, c);
        }
        // col 5 はスペース
        for (i, c) in "world".chars().enumerate() {
            write_char(&mut grid, 0, 6 + i, c);
        }

        let cursor = ViCursor::new(Point::new(Line(0), Column(0)));
        let moved = cursor.motion(&grid, ViMotion::WordRight);
        // "hello" の後の空白をスキップして "world" の先頭 (col 6) に移動
        assert_eq!(
            moved.point.column.0, 6,
            "word_right: expected col 6, got {}",
            moved.point.column.0
        );
    }

    #[test]
    fn vi_cursor_word_left_basic() {
        let mut grid = make_grid(5, 20);
        for (i, c) in "hello".chars().enumerate() {
            write_char(&mut grid, 0, i, c);
        }
        for (i, c) in "world".chars().enumerate() {
            write_char(&mut grid, 0, 6 + i, c);
        }

        // "world" の末尾から b
        let cursor = ViCursor::new(Point::new(Line(0), Column(10)));
        let moved = cursor.motion(&grid, ViMotion::WordLeft);
        assert_eq!(
            moved.point.column.0, 6,
            "word_left: expected col 6, got {}",
            moved.point.column.0
        );
    }

    #[test]
    fn vi_cursor_first_last() {
        let cols = 80;
        let mid_col = 40;
        let last_col = cols - 1;
        let grid = make_grid(5, cols);
        let cursor = ViCursor::new(Point::new(Line(0), Column(mid_col)));
        let moved_first = cursor.motion(&grid, ViMotion::First);
        assert_eq!(moved_first.point.column.0, 0);
        let moved_last = cursor.motion(&grid, ViMotion::Last);
        assert_eq!(moved_last.point.column.0, last_col);
    }

    #[test]
    fn vi_cursor_screen_top_middle_bottom() {
        let grid = make_grid(10, 80);
        let cursor = ViCursor::new(Point::new(Line(5), Column(0)));

        let top = cursor.motion(&grid, ViMotion::ScreenTop);
        assert_eq!(top.point.line.0, 0); // display_offset=0 → Line(0)

        let mid = cursor.motion(&grid, ViMotion::ScreenMiddle);
        assert_eq!(mid.point.line.0, 5); // 0 + 10/2 = 5

        let bot = cursor.motion(&grid, ViMotion::ScreenBottom);
        assert_eq!(bot.point.line.0, 9); // 0 + 10 - 1 = 9
    }
}
