//! Grid 内テキスト検索エンジン。
//!
//! [`SearchEngine`] は `Grid<Cell>` の全行（履歴 + ビューポート）を走査し、
//! 大文字小文字を区別しない部分一致検索を行う。

use crate::grid::{Cell, CellFlags, Grid};

// ---------------------------------------------------------------------------
// SearchMatch
// ---------------------------------------------------------------------------

/// 検索マッチ 1 件の位置情報。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchMatch {
    /// 行番号（raw storage インデックス: 0 = 最古の履歴行）。
    pub raw_row: usize,
    /// マッチ開始列（0-indexed）。
    pub start_col: usize,
    /// マッチ終了列（exclusive, 0-indexed）。
    pub end_col: usize,
}

// ---------------------------------------------------------------------------
// SearchEngine
// ---------------------------------------------------------------------------

/// 検索エンジン。Grid 全体を走査してテキストマッチを検出する。
pub struct SearchEngine;

impl SearchEngine {
    /// Grid 全体を走査し、`query` に一致する位置を返す。
    ///
    /// - 大文字小文字を区別しない（case-insensitive）検索を行う。
    /// - 戻り値の `raw_row` は Grid の raw storage インデックス（0 = 最古の履歴行）。
    /// - `query` が空の場合は空のベクタを返す。
    pub fn search(grid: &Grid<Cell>, query: &str) -> Vec<SearchMatch> {
        if query.is_empty() {
            return Vec::new();
        }

        let query_lower = query.to_lowercase();
        let mut results = Vec::new();
        const MAX_MATCHES: usize = 10_000;

        for (raw_row, row) in grid.iter_raw_rows().enumerate() {
            // セル列をテキストに変換（wide spacer をスキップ）。
            // cells と char インデックスのマッピングも同時に構築する。
            let cells = row.cells();
            let (text, col_map) = cells_to_text_with_map(cells);

            let text_lower = text.to_lowercase();

            // 部分一致検索（重複マッチを許すために文字単位で進める）。
            let mut search_start = 0usize; // バイトオフセット
            while search_start <= text_lower.len().saturating_sub(query_lower.len()) {
                if let Some(rel_offset) = text_lower[search_start..].find(&query_lower[..]) {
                    let byte_start = search_start + rel_offset;
                    let byte_end = byte_start + query_lower.len();

                    let start_col = byte_offset_to_col(&text, &col_map, byte_start);
                    let end_col = byte_offset_to_col(&text, &col_map, byte_end);

                    results.push(SearchMatch { raw_row, start_col, end_col });
                    if results.len() >= MAX_MATCHES {
                        return results;
                    }

                    // 次の検索開始位置を 1 文字進める（重複マッチ用）。
                    // byte_start の文字の長さ分だけ進める。
                    let ch_len = text[byte_start..].chars().next().map_or(1, char::len_utf8);
                    search_start = byte_start + ch_len;
                } else {
                    break;
                }
            }
        }

        results
    }

    /// `raw_row` をビューポート相対の行番号に変換する。
    ///
    /// - `display_offset == 0`（ライブビューポート）の場合:
    ///   `viewport_row = raw_row - history_size`
    /// - `display_offset > 0`（履歴スクロール中）の場合:
    ///   表示領域は `[history_size - display_offset, history_size - display_offset + screen_lines)`
    ///   よって `viewport_row = raw_row - (history_size - display_offset)`
    ///
    /// ビューポート外の場合は `None` を返す。
    pub fn raw_row_to_viewport(
        raw_row: usize,
        history_size: usize,
        display_offset: usize,
        screen_lines: usize,
    ) -> Option<usize> {
        // 現在の表示領域の先頭 raw_row。
        let view_start = history_size.saturating_sub(display_offset);
        let view_end = view_start + screen_lines;

        if raw_row >= view_start && raw_row < view_end { Some(raw_row - view_start) } else { None }
    }

    /// マッチ位置がビューポートに表示されるよう `display_offset` を計算する。
    ///
    /// マッチ行がビューポートの中央付近に表示されるよう調整する。
    /// 返り値は設定すべき `display_offset` の値。
    pub fn display_offset_for_match(
        raw_row: usize,
        history_size: usize,
        screen_lines: usize,
    ) -> usize {
        if raw_row >= history_size {
            // ビューポート内: スクロール不要。
            return 0;
        }

        // 履歴行の場合: マッチ行を中央付近に表示する。
        let half = screen_lines / 2;
        // raw_row をビューポートの中央に配置するオフセット。
        // display_offset は history_size からの逆算。
        // view_start = history_size - display_offset
        // 中央配置: raw_row = view_start + half
        //           view_start = raw_row.saturating_sub(half)
        //           display_offset = history_size - view_start
        let view_start = raw_row.saturating_sub(half);
        history_size.saturating_sub(view_start)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// セル配列をテキストに変換しながら、テキストのバイトオフセット → セル列インデックスの
/// マッピングも返す。
///
/// `col_map[i]` = テキストの i 番目の文字が由来するセル列インデックス。
fn cells_to_text_with_map(cells: &[Cell]) -> (String, Vec<usize>) {
    let mut text = String::new();
    let mut col_map: Vec<usize> = Vec::new();

    for (col, cell) in cells.iter().enumerate() {
        if cell.flags.contains(CellFlags::WIDE_CHAR_SPACER) {
            continue;
        }
        let ch = cell.c;
        // 各文字のバイト長に対応するエントリを col_map に追加する。
        let byte_len = ch.len_utf8();
        // col_map はバイト単位ではなく文字単位で管理する（後で char 位置から列を引く）。
        // ここでは文字 1 つにつき 1 エントリ。
        let _ = byte_len; // byte_len は後続の byte_offset_to_col で使われる
        col_map.push(col);
        text.push(ch);
    }

    (text, col_map)
}

/// テキスト内のバイトオフセットを、対応するセル列インデックスに変換する。
///
/// `col_map[char_idx]` = セル列インデックス。
fn byte_offset_to_col(text: &str, col_map: &[usize], byte_offset: usize) -> usize {
    // バイトオフセットを文字インデックスに変換する。
    let char_idx = text[..byte_offset].chars().count();
    col_map.get(char_idx).copied().unwrap_or(col_map.len())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::{Column, Line, Point};

    /// 指定サイズの Grid を作成し、viewport 行の指定位置にテキストを書き込む。
    fn make_grid_with_text(lines: usize, cols: usize, row: usize, text: &str) -> Grid<Cell> {
        let mut grid: Grid<Cell> = Grid::new(lines, cols, 200);
        for (i, ch) in text.chars().enumerate() {
            if i >= cols {
                break;
            }
            grid[Point::new(Line(i32::try_from(row).unwrap_or(0)), Column(i))].c = ch;
        }
        grid
    }

    #[test]
    fn search_empty_query() {
        let grid = make_grid_with_text(5, 40, 0, "hello world");
        let results = SearchEngine::search(&grid, "");
        assert!(results.is_empty());
    }

    #[test]
    fn search_no_match() {
        let grid = make_grid_with_text(5, 40, 0, "hello world");
        let results = SearchEngine::search(&grid, "zzz");
        assert!(results.is_empty());
    }

    #[test]
    fn search_single_match() {
        let grid = make_grid_with_text(5, 40, 0, "hello world");
        let results = SearchEngine::search(&grid, "world");
        assert_eq!(results.len(), 1);
        let m = &results[0];
        // "hello world" => "world" は col 6 から始まる
        let expected_start = "hello ".len();
        let expected_end = expected_start + "world".len();
        assert_eq!(m.start_col, expected_start);
        assert_eq!(m.end_col, expected_end);
    }

    #[test]
    fn search_multiple_matches_same_row() {
        // "abab" を検索すると "ab" が 2 回マッチ
        let grid = make_grid_with_text(5, 40, 0, "abab");
        let results = SearchEngine::search(&grid, "ab");
        assert_eq!(results.len(), 2, "should find 2 non-overlapping matches");
        assert_eq!(results[0].start_col, 0, "first match starts at col 0");
        assert_eq!(results[0].end_col, 2, "first match ends at col 2");
        assert_eq!(results[1].start_col, 2, "second match starts at col 2");
        assert_eq!(results[1].end_col, 4, "second match ends at col 4");
    }

    #[test]
    fn search_multiple_matches_multiple_rows() {
        let mut grid: Grid<Cell> = Grid::new(5, 40, 200);
        // 行 0 に "foo bar"
        for (i, ch) in "foo bar".chars().enumerate() {
            grid[Point::new(Line(0), Column(i))].c = ch;
        }
        // 行 2 に "bar baz"
        for (i, ch) in "bar baz".chars().enumerate() {
            grid[Point::new(Line(2), Column(i))].c = ch;
        }

        let results = SearchEngine::search(&grid, "bar");
        assert_eq!(results.len(), 2);
        // 両方とも同じ raw_row ではなく異なる行にある
        assert_ne!(results[0].raw_row, results[1].raw_row);
        // start_col の確認
        assert_eq!(results[0].start_col, 4); // "foo bar" の bar は col 4
        assert_eq!(results[1].start_col, 0); // "bar baz" の bar は col 0
    }

    #[test]
    fn search_case_insensitive() {
        let grid = make_grid_with_text(5, 40, 0, "Hello World");
        // 小文字で検索
        let results = SearchEngine::search(&grid, "hello");
        assert_eq!(results.len(), 1, "lowercase 'hello' should match once");
        assert_eq!(results[0].start_col, 0, "lowercase match starts at col 0");
        assert_eq!(results[0].end_col, "hello".len(), "lowercase match end col");

        // 大文字で検索
        let results2 = SearchEngine::search(&grid, "WORLD");
        assert_eq!(results2.len(), 1, "uppercase 'WORLD' should match once");
        let world_start = "Hello ".len();
        assert_eq!(results2[0].start_col, world_start, "uppercase match start col");
        assert_eq!(results2[0].end_col, world_start + "WORLD".len(), "uppercase match end col");
    }

    #[test]
    fn raw_row_to_viewport_live() {
        // history_size=5, display_offset=0, screen_lines=10
        // view_start = 5 - 0 = 5
        // raw_row 5 → viewport 0, raw_row 14 → viewport 9
        let result = SearchEngine::raw_row_to_viewport(5, 5, 0, 10);
        assert_eq!(result, Some(0));

        let result2 = SearchEngine::raw_row_to_viewport(14, 5, 0, 10);
        assert_eq!(result2, Some(9));

        // ビューポート外（履歴行）
        let result3 = SearchEngine::raw_row_to_viewport(4, 5, 0, 10);
        assert_eq!(result3, None);

        // ビューポート外（後方）
        let result4 = SearchEngine::raw_row_to_viewport(15, 5, 0, 10);
        assert_eq!(result4, None);
    }

    #[test]
    fn raw_row_to_viewport_scrolled() {
        // history_size=10, display_offset=5, screen_lines=10
        // view_start = 10 - 5 = 5
        // raw_row 5 → viewport 0, raw_row 14 → viewport 9
        let result = SearchEngine::raw_row_to_viewport(5, 10, 5, 10);
        assert_eq!(result, Some(0));

        let result2 = SearchEngine::raw_row_to_viewport(14, 10, 5, 10);
        assert_eq!(result2, Some(9));

        // ビューポート外
        let result3 = SearchEngine::raw_row_to_viewport(4, 10, 5, 10);
        assert_eq!(result3, None);
    }

    #[test]
    fn display_offset_for_match_in_history() {
        let history_size = 20;
        let screen_lines = 10;
        // raw_row=3 (履歴行の先頭付近)
        // half=5, view_start = max(3-5, 0) = 0
        // display_offset = history_size - 0 = 20
        let raw_row = 3;
        let offset = SearchEngine::display_offset_for_match(raw_row, history_size, screen_lines);
        assert_eq!(offset, history_size);

        // raw_row=10 (履歴行の中間)
        // view_start = 10 - 5 = 5
        // display_offset = history_size - 5 = 15
        let raw_row2 = 10;
        let offset2 = SearchEngine::display_offset_for_match(raw_row2, history_size, screen_lines);
        assert_eq!(offset2, history_size - (raw_row2 - screen_lines / 2));
    }

    #[test]
    fn display_offset_for_match_in_viewport() {
        // raw_row >= history_size → offset = 0
        let offset = SearchEngine::display_offset_for_match(20, 20, 10);
        assert_eq!(offset, 0);

        let offset2 = SearchEngine::display_offset_for_match(25, 20, 10);
        assert_eq!(offset2, 0);
    }

    #[test]
    fn search_skips_empty_rows() {
        // 空行が多数ある状態でも正しく検索できることを確認する。
        // 5 行のグリッドで最初と最後の行にだけ内容を書き込む。
        let grid = {
            let mut g: Grid<Cell> = Grid::new(5, 40, 200);
            for (i, ch) in "alpha".chars().enumerate() {
                g[Point::new(Line(0), Column(i))].c = ch;
            }
            for (i, ch) in "beta".chars().enumerate() {
                g[Point::new(Line(4), Column(i))].c = ch;
            }
            g
        };

        let results = SearchEngine::search(&grid, "a");
        // "alpha" の 'a' は col 0 と col 4 の 2 件
        // "beta" の 'a' は col 3 の 1 件
        assert!(results.len() >= 3);

        // "alpha" のマッチは row 0 から始まる
        let alpha_matches: Vec<_> = results.iter().filter(|m| m.start_col == 0).collect();
        assert!(!alpha_matches.is_empty());

        // "beta" のマッチは row 4 (= raw_row: history_size + 4 = 4) に存在する
        let beta_matches: Vec<_> = results.iter().filter(|m| m.start_col == 3).collect();
        assert!(!beta_matches.is_empty());

        // 2 つのグループは異なる raw_row にある
        assert_ne!(alpha_matches[0].raw_row, beta_matches[0].raw_row);
    }
}
