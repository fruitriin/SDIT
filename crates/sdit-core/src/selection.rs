//! テキスト選択の型と選択範囲のテキスト抽出。
//!
//! [`Selection`] は選択タイプ（Simple / Word / Lines）と
//! 開始・終了 [`Point`] を保持する。
//! [`selected_text`] は選択範囲のテキストを Grid から抽出する。

use crate::grid::{Cell, Dimensions, Grid};
use crate::index::{Column, Line, Point};

// ---------------------------------------------------------------------------
// SelectionType
// ---------------------------------------------------------------------------

/// 選択タイプ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionType {
    /// 通常のドラッグ選択。
    Simple,
    /// 単語選択（ダブルクリック）。
    Word,
    /// 行選択（トリプルクリック）。
    Lines,
}

// ---------------------------------------------------------------------------
// Selection
// ---------------------------------------------------------------------------

/// テキスト選択の範囲。
#[derive(Debug, Clone)]
pub struct Selection {
    /// 選択タイプ。
    pub ty: SelectionType,
    /// 選択の開始点（グリッド座標）。
    pub start: Point,
    /// 選択の終了点（グリッド座標）。ドラッグ中に更新される。
    pub end: Point,
}

impl Selection {
    /// 新しい選択を指定点から作成する。
    pub fn new(ty: SelectionType, point: Point) -> Self {
        Self { ty, start: point, end: point }
    }

    /// 選択範囲を正規化して `(start, end)` を返す。
    ///
    /// `start` が常に `end` より前（または同じ位置）になるように整列する。
    pub fn normalized(&self) -> (Point, Point) {
        if self.start <= self.end { (self.start, self.end) } else { (self.end, self.start) }
    }

    /// セル `(col, row)` が選択範囲内かどうかを判定する。
    ///
    /// `row` はビューポートインデックス（0-based）。
    pub fn contains(&self, col: usize, row: usize) -> bool {
        let (start, end) = self.normalized();
        let sr = start.line.as_viewport_idx();
        let sc = start.column.0;
        let er = end.line.as_viewport_idx();
        let ec = end.column.0;

        if self.ty == SelectionType::Lines {
            return row >= sr && row <= er;
        }
        if row < sr || row > er {
            return false;
        }
        if sr == er {
            // 同一行: 列範囲内
            col >= sc && col <= ec
        } else if row == sr {
            col >= sc
        } else if row == er {
            col <= ec
        } else {
            // 中間行: 全列
            true
        }
    }

    /// 選択範囲を `((start_col, start_row), (end_col, end_row))` タプルに変換する。
    ///
    /// `Lines` タイプの場合は行全体（col は 0 から最大値）を示すタプルを返す。
    /// pipeline の `is_in_selection()` に渡す用途に使う。
    pub fn to_tuple(&self, grid_cols: usize) -> ((usize, usize), (usize, usize)) {
        let (s, e) = self.normalized();
        let sr = s.line.as_viewport_idx();
        let er = e.line.as_viewport_idx();
        match self.ty {
            SelectionType::Lines => ((0, sr), (grid_cols.saturating_sub(1), er)),
            _ => ((s.column.0, sr), (e.column.0, er)),
        }
    }
}

// ---------------------------------------------------------------------------
// selected_text
// ---------------------------------------------------------------------------

/// 選択範囲のテキストを Grid から抽出する。
///
/// 各行末の空白はトリムされる。行間は `\n` で区切られる。
/// 最後の行には `\n` を付けない（ただし `Lines` タイプは最終行にも `\n` を付ける）。
pub fn selected_text(grid: &Grid<Cell>, selection: &Selection) -> String {
    let (start, end) = selection.normalized();
    let sr = start.line.as_viewport_idx();
    let sc = start.column.0;
    let er = end.line.as_viewport_idx();
    let ec = end.column.0;
    let cols = grid.columns();
    let screen_lines = grid.screen_lines();
    let mut result = String::new();

    for row in sr..=er {
        if row >= screen_lines {
            break;
        }
        let start_col = if row == sr { sc } else { 0 };
        let end_col = if row == er { ec } else { cols.saturating_sub(1) };

        // 行テキストを取得
        let mut line_text = String::new();
        for col in start_col..=end_col {
            // row は screen_lines の範囲内なので i32 に収まる。
            #[allow(clippy::cast_possible_wrap)]
            let point = crate::index::Point::new(Line(row as i32), Column(col));
            let cell = &grid[point];
            line_text.push(cell.c);
        }

        // 行末空白をトリム（Lines タイプでは全行、それ以外は中間行と最終行以外）
        let trimmed = line_text.trim_end();
        result.push_str(trimmed);

        // 改行を付加
        let is_last_row = row == er;
        if !is_last_row || selection.ty == SelectionType::Lines {
            result.push('\n');
        }
    }

    // Lines タイプで末尾に余分な改行がある場合は除去
    if selection.ty == SelectionType::Lines && result.ends_with('\n') {
        result.pop();
    }

    result
}

// ---------------------------------------------------------------------------
// テスト
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::Grid;
    use crate::index::{Column, Line, Point};

    fn write_str(grid: &mut Grid<Cell>, row: usize, col: usize, s: &str) {
        for (i, c) in s.chars().enumerate() {
            #[allow(clippy::cast_possible_wrap)]
            let pt = Point::new(Line(row as i32), Column(col + i));
            grid[pt].c = c;
        }
    }

    // --- Selection::contains テスト ---

    #[test]
    fn contains_simple_single_row() {
        let sel = Selection {
            ty: SelectionType::Simple,
            start: Point::new(Line(1), Column(2)),
            end: Point::new(Line(1), Column(5)),
        };
        // 範囲内
        assert!(sel.contains(2, 1), "start col should be in range");
        assert!(sel.contains(5, 1), "end col should be in range");
        assert!(sel.contains(3, 1), "mid col should be in range");
        // 範囲外
        assert!(!sel.contains(1, 1), "col before start should be out");
        assert!(!sel.contains(6, 1), "col after end should be out");
        assert!(!sel.contains(3, 0), "row above should be out");
        assert!(!sel.contains(3, 2), "row below should be out");
    }

    #[test]
    fn contains_simple_multi_row() {
        let sel = Selection {
            ty: SelectionType::Simple,
            start: Point::new(Line(1), Column(3)),
            end: Point::new(Line(3), Column(5)),
        };
        // 開始行: sc以降
        assert!(sel.contains(3, 1), "start col on start row");
        assert!(sel.contains(9, 1), "col after start on start row");
        assert!(!sel.contains(2, 1), "col before start on start row");
        // 中間行: 全列
        assert!(sel.contains(0, 2), "first col on middle row");
        assert!(sel.contains(99, 2), "far col on middle row");
        // 終了行: ec以前
        assert!(sel.contains(0, 3), "first col on end row");
        assert!(sel.contains(5, 3), "end col on end row");
        assert!(!sel.contains(6, 3), "col after end on end row");
    }

    #[test]
    fn contains_reversed_selection() {
        // end < start でも normalized() で正しく動作する
        let sel = Selection {
            ty: SelectionType::Simple,
            start: Point::new(Line(3), Column(5)),
            end: Point::new(Line(1), Column(3)),
        };
        assert!(sel.contains(3, 1));
        assert!(sel.contains(5, 3));
        assert!(!sel.contains(2, 1));
    }

    #[test]
    fn contains_lines_type() {
        let sel = Selection {
            ty: SelectionType::Lines,
            start: Point::new(Line(2), Column(0)),
            end: Point::new(Line(4), Column(79)),
        };
        // 行全体が選択される
        assert!(sel.contains(0, 2), "first col on start row");
        assert!(sel.contains(79, 2), "last col on start row");
        assert!(sel.contains(0, 4), "first col on end row");
        assert!(!sel.contains(0, 1), "row before selection");
        assert!(!sel.contains(0, 5), "row after selection");
    }

    // --- selected_text テスト ---

    #[test]
    fn selected_text_single_row() {
        let mut grid = Grid::new(5, 10, 0);
        write_str(&mut grid, 0, 0, "Hello     ");
        let sel = Selection {
            ty: SelectionType::Simple,
            start: Point::new(Line(0), Column(0)),
            end: Point::new(Line(0), Column(4)),
        };
        let text = selected_text(&grid, &sel);
        assert_eq!(text, "Hello");
    }

    #[test]
    fn selected_text_multi_row() {
        let mut grid = Grid::new(5, 10, 0);
        write_str(&mut grid, 0, 0, "Hello");
        write_str(&mut grid, 1, 0, "World");
        let sel = Selection {
            ty: SelectionType::Simple,
            start: Point::new(Line(0), Column(0)),
            end: Point::new(Line(1), Column(4)),
        };
        let text = selected_text(&grid, &sel);
        assert_eq!(text, "Hello\nWorld");
    }

    #[test]
    fn selected_text_lines_type() {
        let mut grid = Grid::new(5, 10, 0);
        write_str(&mut grid, 1, 0, "abc");
        write_str(&mut grid, 2, 0, "def");
        let sel = Selection {
            ty: SelectionType::Lines,
            start: Point::new(Line(1), Column(0)),
            end: Point::new(Line(2), Column(9)),
        };
        let text = selected_text(&grid, &sel);
        // Lines タイプは行全体
        assert!(text.contains("abc"));
        assert!(text.contains("def"));
        // 末尾改行は除去
        assert!(!text.ends_with('\n'));
    }

    #[test]
    fn selected_text_trims_trailing_spaces() {
        let mut grid = Grid::new(5, 10, 0);
        write_str(&mut grid, 0, 0, "Hi   ");
        let sel = Selection {
            ty: SelectionType::Simple,
            start: Point::new(Line(0), Column(0)),
            end: Point::new(Line(0), Column(9)),
        };
        let text = selected_text(&grid, &sel);
        assert_eq!(text, "Hi");
    }

    #[test]
    fn to_tuple_simple() {
        let sel = Selection {
            ty: SelectionType::Simple,
            start: Point::new(Line(1), Column(3)),
            end: Point::new(Line(2), Column(7)),
        };
        assert_eq!(sel.to_tuple(80), ((3, 1), (7, 2)));
    }

    #[test]
    fn to_tuple_lines() {
        let sel = Selection {
            ty: SelectionType::Lines,
            start: Point::new(Line(1), Column(0)),
            end: Point::new(Line(3), Column(79)),
        };
        assert_eq!(sel.to_tuple(80), ((0, 1), (79, 3)));
    }
}
