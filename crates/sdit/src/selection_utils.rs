//! テキスト選択ユーティリティ。

use sdit_core::index::{Column, Line, Point};

/// グリッドの `(row, col)` から単語の開始・終了列インデックスを返す。
///
/// 空白・記号を区切り文字として扱い、連続する英数字・その他の文字を「単語」とみなす。
/// `word_chars` が空でない場合は、それらの文字も単語の一部として扱う。
pub(crate) fn expand_word(
    grid: &sdit_core::grid::Grid<sdit_core::grid::Cell>,
    row: usize,
    col: usize,
    word_chars: &str,
) -> (usize, usize) {
    use sdit_core::grid::Dimensions as _;

    let cols = grid.columns();
    if cols == 0 {
        return (col, col);
    }
    let col = col.min(cols - 1);

    // 区切り文字セット（word_chars に含まれる文字は区切り文字から除外）
    let is_separator = |c: char| {
        if !word_chars.is_empty() && word_chars.contains(c) {
            return false;
        }
        c.is_ascii_whitespace() || " \t!@#$%^&*()-=+[]{}|;:'\",.<>?/\\`~".contains(c)
    };

    // 起点セルの文字を取得
    #[allow(clippy::cast_possible_wrap)]
    let origin_cell = &grid[Point::new(Line(row as i32), Column(col))];
    let origin_is_sep = is_separator(origin_cell.c);

    // 左方向に拡張
    let mut start = col;
    loop {
        if start == 0 {
            break;
        }
        #[allow(clippy::cast_possible_wrap)]
        let c = grid[Point::new(Line(row as i32), Column(start - 1))].c;
        if is_separator(c) != origin_is_sep {
            break;
        }
        start -= 1;
    }

    // 右方向に拡張
    let mut end = col;
    loop {
        if end + 1 >= cols {
            break;
        }
        #[allow(clippy::cast_possible_wrap)]
        let c = grid[Point::new(Line(row as i32), Column(end + 1))].c;
        if is_separator(c) != origin_is_sep {
            break;
        }
        end += 1;
    }

    (start, end)
}
