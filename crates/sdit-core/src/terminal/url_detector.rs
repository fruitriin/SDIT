//! URL 検出モジュール。
//!
//! OSC 8 ハイパーリンク優先で、なければ正規表現でセル行中の URL を検出する。

use regex::Regex;

use crate::grid::Cell;

/// URL マッチ結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UrlMatch {
    /// 検出された URL 文字列。
    pub url: String,
    /// URL が始まる列位置（0-indexed）。
    pub start_col: usize,
    /// URL が終わる列位置（0-indexed, exclusive）。
    pub end_col: usize,
}

/// URL 検出器。コンパイル済み正規表現を保持する。
pub struct UrlDetector {
    regex: Regex,
}

impl UrlDetector {
    /// 新しい `UrlDetector` を作成する。
    ///
    /// 正規表現はコンパイルに成功することが保証されているため `unwrap` する。
    pub fn new() -> Self {
        // https?:// で始まり、空白・制御文字・一部の区切り文字で終わるパターン
        // raw string 内では \" が使えないため " を別途除外する
        let regex = Regex::new(r#"https?://[^\s<>"{}|\\^\[\]]*[^\s<>"{}|\\^\[\].,;:!?'()]"#)
            .expect("URL regex compile failed");
        Self { regex }
    }

    /// セルの配列からテキストを構築し、含まれる URL の一覧を返す。
    pub fn detect_urls_in_line(&self, cells: &[Cell]) -> Vec<UrlMatch> {
        let text = cells_to_text(cells);
        self.regex
            .find_iter(&text)
            .map(|m| {
                let start_col = byte_offset_to_col(&text, cells, m.start());
                let end_col = byte_offset_to_col(&text, cells, m.end());
                UrlMatch { url: m.as_str().to_owned(), start_col, end_col }
            })
            .collect()
    }

    /// 指定列位置に URL があるか判定する。
    ///
    /// OSC 8 hyperlink が優先（`Cell.hyperlink` が `Some` なら即返す）。
    /// なければ正規表現で行全体をスキャンして列位置が範囲内のものを返す。
    pub fn find_url_at(&self, cells: &[Cell], col: usize) -> Option<String> {
        // OSC 8 優先
        if let Some(cell) = cells.get(col) {
            if let Some(ref url) = cell.hyperlink {
                // ディフェンスインデプス: OSC 8 パーサーで http/https に制限済みだが再確認
                let u = url.as_ref();
                if u.starts_with("http://") || u.starts_with("https://") {
                    return Some(u.to_owned());
                }
            }
        }

        // 正規表現でスキャン（正規表現パターン自体が https?:// に制限済み）
        let matches = self.detect_urls_in_line(cells);
        matches.into_iter().find(|m| col >= m.start_col && col < m.end_col).map(|m| m.url)
    }
}

impl Default for UrlDetector {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// セル配列を文字列に変換する。Wide spacer セルはスキップする。
fn cells_to_text(cells: &[Cell]) -> String {
    use crate::grid::CellFlags;
    cells.iter().filter(|c| !c.flags.contains(CellFlags::WIDE_CHAR_SPACER)).map(|c| c.c).collect()
}

/// テキスト内のバイトオフセットを、対応する cells のセル列インデックスに変換する。
///
/// `cells_to_text` でスキップした spacer を考慮して列番号を計算する。
fn byte_offset_to_col(text: &str, cells: &[Cell], byte_offset: usize) -> usize {
    use crate::grid::CellFlags;
    // テキスト内の文字数に変換
    let char_pos = text[..byte_offset].chars().count();
    // char_pos 番目の「non-spacer」セルの cells インデックスを求める
    let mut count = 0usize;
    for (i, cell) in cells.iter().enumerate() {
        if cell.flags.contains(CellFlags::WIDE_CHAR_SPACER) {
            continue;
        }
        if count == char_pos {
            return i;
        }
        count += 1;
    }
    cells.len()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::grid::Cell;

    fn make_cells(text: &str) -> Vec<Cell> {
        text.chars().map(|c| Cell { c, ..Cell::default() }).collect()
    }

    fn make_cells_with_hyperlink(text: &str, url: &str) -> Vec<Cell> {
        let arc_url: Arc<str> = Arc::from(url);
        text.chars()
            .map(|c| Cell { c, hyperlink: Some(Arc::clone(&arc_url)), ..Cell::default() })
            .collect()
    }

    #[test]
    fn detect_simple_url() {
        let detector = UrlDetector::new();
        let cells = make_cells("visit https://example.com for more");
        let matches = detector.detect_urls_in_line(&cells);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].url, "https://example.com");
        assert_eq!(matches[0].start_col, 6);
        assert_eq!(matches[0].end_col, 25);
    }

    #[test]
    fn detect_multiple_urls() {
        let detector = UrlDetector::new();
        let cells = make_cells("https://a.com and https://b.com end");
        let matches = detector.detect_urls_in_line(&cells);
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].url, "https://a.com");
        assert_eq!(matches[1].url, "https://b.com");
    }

    #[test]
    fn detect_no_url() {
        let detector = UrlDetector::new();
        let cells = make_cells("no urls here");
        assert!(detector.detect_urls_in_line(&cells).is_empty());
    }

    #[test]
    fn find_url_at_osc8_priority() {
        let detector = UrlDetector::new();
        // OSC 8 hyperlink を持つセルの場合、正規表現より優先される
        let mut cells = make_cells("click here to visit");
        // col 6 のセルに hyperlink をセット
        cells[6].hyperlink = Some(Arc::from("https://osc8.example.com"));
        let result = detector.find_url_at(&cells, 6);
        assert_eq!(result, Some("https://osc8.example.com".to_owned()));
    }

    #[test]
    fn find_url_at_regex_fallback() {
        let detector = UrlDetector::new();
        let cells = make_cells("see https://example.com now");
        // col 4 は https の 's' の位置
        let result = detector.find_url_at(&cells, 4);
        assert_eq!(result, Some("https://example.com".to_owned()));
    }

    #[test]
    fn find_url_at_not_in_url() {
        let detector = UrlDetector::new();
        let cells = make_cells("see https://example.com now");
        // col 0 は URL 外
        let result = detector.find_url_at(&cells, 0);
        assert!(result.is_none());
    }

    #[test]
    fn osc8_hyperlink_all_cells() {
        let detector = UrlDetector::new();
        let cells = make_cells_with_hyperlink("hello", "https://osc8.example.com");
        for col in 0..5 {
            let result = detector.find_url_at(&cells, col);
            assert_eq!(result, Some("https://osc8.example.com".to_owned()), "col {col}");
        }
    }

    #[test]
    fn url_with_path_and_query() {
        let detector = UrlDetector::new();
        let cells = make_cells("https://example.com/path?q=1&r=2");
        let matches = detector.detect_urls_in_line(&cells);
        assert_eq!(matches.len(), 1);
        assert!(matches[0].url.contains("/path?q=1&r=2"));
    }

    #[test]
    fn http_url_detected() {
        let detector = UrlDetector::new();
        let cells = make_cells("http://example.com");
        let matches = detector.detect_urls_in_line(&cells);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].url, "http://example.com");
    }
}
