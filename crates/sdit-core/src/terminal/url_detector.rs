//! URL 検出モジュール。
//!
//! OSC 8 ハイパーリンク優先で、なければ正規表現でセル行中の URL を検出する。
//! また、QuickSelect 用の汎用パターンマッチングも提供する。

use std::sync::OnceLock;

use regex::{Captures, Regex, RegexBuilder};

use crate::config::LinkConfig;
use crate::grid::Cell;

/// URL マッチ結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UrlMatch {
    /// 検出された URL 文字列（カスタムリンクの場合はテンプレート展開済み）。
    pub url: String,
    /// URL が始まる列位置（0-indexed）。
    pub start_col: usize,
    /// URL が終わる列位置（0-indexed, exclusive）。
    pub end_col: usize,
}

/// 汎用パターンマッチ結果（QuickSelect 用）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternMatch {
    /// マッチしたテキスト文字列。
    pub text: String,
    /// マッチが始まる列位置（0-indexed）。
    pub start_col: usize,
    /// マッチが終わる列位置（0-indexed, exclusive）。
    pub end_col: usize,
}

/// URL 検出器。コンパイル済み正規表現を保持する。
pub struct UrlDetector {
    regex: Regex,
    /// カスタムリンクパターン: (コンパイル済み正規表現, action テンプレート文字列)。
    custom_patterns: Vec<(Regex, String)>,
}

impl UrlDetector {
    /// 新しい `UrlDetector` を作成する（カスタムリンクなし）。
    ///
    /// 正規表現はコンパイルに成功することが保証されているため `unwrap` する。
    pub fn new() -> Self {
        Self::with_links(&[])
    }

    /// カスタムリンク設定を指定して `UrlDetector` を作成する。
    ///
    /// 各 `LinkConfig` の regex をコンパイルし、失敗した場合はログを出してスキップする。
    /// ReDoS 対策として `size_limit` を 1 MiB に制限する。
    pub fn with_links(links: &[LinkConfig]) -> Self {
        // https?:// で始まり、空白・制御文字・一部の区切り文字で終わるパターン
        // raw string 内では \" が使えないため " を別途除外する
        let regex = Regex::new(r#"https?://[^\s<>"{}|\\^\[\]]*[^\s<>"{}|\\^\[\].,;:!?'()]"#)
            .expect("URL regex compile failed");

        let custom_patterns = links
            .iter()
            .filter_map(|lc| match RegexBuilder::new(&lc.regex).size_limit(1 << 20).build() {
                Ok(re) => Some((re, lc.action.clone())),
                Err(e) => {
                    log::warn!("Custom link regex compile error '{}': {e}", lc.regex);
                    None
                }
            })
            .collect();

        Self { regex, custom_patterns }
    }

    /// セルの配列からテキストを構築し、含まれる URL の一覧を返す。
    ///
    /// 組み込み URL パターン（https?://...）のみを検索する。
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
    /// なければカスタムパターン、次に組み込み URL 正規表現で行全体をスキャンする。
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

        let text = cells_to_text(cells);

        // カスタムパターンをスキャン
        for (re, action_template) in &self.custom_patterns {
            for caps in re.captures_iter(&text) {
                let m = caps.get(0).expect("capture 0 always exists");
                let start_col = byte_offset_to_col(&text, cells, m.start());
                let end_col = byte_offset_to_col(&text, cells, m.end());
                if col >= start_col && col < end_col {
                    if let Some(url) = extract_url_from_action(action_template, &caps) {
                        return Some(url);
                    }
                }
            }
        }

        // 組み込み URL 正規表現でスキャン
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
// カスタムリンクテンプレート展開
// ---------------------------------------------------------------------------

/// action テンプレートから URL を展開する。
///
/// `"open:<URL_TEMPLATE>"` 形式の action を解析し、
/// `$0` → マッチ全体、`$1` 等 → キャプチャグループに置換した URL を返す。
/// `open:` プレフィックスがない場合や危険なスキームの場合は `None` を返す。
pub fn extract_url_from_action(action: &str, captures: &Captures<'_>) -> Option<String> {
    let url_template = action.strip_prefix("open:")?;
    let expanded = expand_template(url_template, captures);

    // javascript: / data: スキームは XSS 防止のため拒否
    let lower = expanded.to_ascii_lowercase();
    if lower.starts_with("javascript:") || lower.starts_with("data:") {
        log::warn!(
            "Custom link rejected dangerous scheme: {}",
            &expanded[..expanded.len().min(64)]
        );
        return None;
    }

    Some(expanded)
}

/// テンプレート文字列を展開する。
///
/// - `$0` → キャプチャ全体（`captures.get(0)`）
/// - `$1`, `$2`, ... → 各キャプチャグループ（存在しない場合は空文字列）
pub fn expand_template(template: &str, captures: &Captures<'_>) -> String {
    let mut result = String::with_capacity(template.len());
    let mut chars = template.char_indices().peekable();

    while let Some((_, ch)) = chars.next() {
        if ch == '$' {
            // 続く数字を読む
            let mut num_str = String::new();
            while let Some(&(_, d)) = chars.peek() {
                if d.is_ascii_digit() {
                    num_str.push(d);
                    chars.next();
                } else {
                    break;
                }
            }
            if num_str.is_empty() {
                result.push('$');
            } else {
                let idx: usize = num_str.parse().unwrap_or(usize::MAX);
                let replacement = captures.get(idx).map_or("", |m| m.as_str());
                result.push_str(replacement);
            }
        } else {
            result.push(ch);
        }
    }

    result
}

// ---------------------------------------------------------------------------
// QuickSelect パターン検出
// ---------------------------------------------------------------------------

/// デフォルトの QuickSelect パターン一覧を返す。
///
/// URL（https?://...）、Unix ファイルパス（/...）、git ハッシュ（7-40桁の16進数）、
/// 数値（IP アドレス含む）を対象とする。
///
/// `OnceLock` によりプロセス内で1回だけコンパイルし、以降は参照を返す。
pub fn default_quick_select_patterns() -> &'static [Regex] {
    static PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        vec![
            // URL
            Regex::new(r#"https?://[^\s<>"{}|\\^\[\]]*[^\s<>"{}|\\^\[\].,;:!?'()]"#)
                .expect("URL pattern compile failed"),
            // Unix ファイルパス（/ で始まり空白以外の文字列）
            Regex::new(r"/[^\s]+").expect("file path pattern compile failed"),
            // git ハッシュ（7〜40桁の16進数。単語境界で区切る）
            Regex::new(r"\b[0-9a-fA-F]{7,40}\b").expect("git hash pattern compile failed"),
            // 数値（IP アドレスを含む。整数・小数・コロン区切りポート番号等）
            Regex::new(r"\b\d+(?:\.\d+){0,3}(?::\d+)?\b").expect("number pattern compile failed"),
        ]
    })
}

/// セル配列に対して複数パターンをマッチし、重複なしで結果を返す。
///
/// 同一範囲のセルに複数のパターンがマッチした場合、先のパターン（より具体的なもの）が優先される。
/// `patterns` が空の場合は `default_quick_select_patterns()` を使用する。
pub fn detect_patterns_in_line(cells: &[Cell], patterns: &[&Regex]) -> Vec<PatternMatch> {
    let text = cells_to_text(cells);
    if text.is_empty() {
        return Vec::new();
    }

    // patterns が空のときは静的デフォルトを参照スライスに変換して使う
    let default_refs: Vec<&Regex>;
    let active_patterns: &[&Regex] = if patterns.is_empty() {
        default_refs = default_quick_select_patterns().iter().collect();
        &default_refs
    } else {
        patterns
    };

    // 各パターンでマッチを収集し、重複排除（先に追加されたものが優先）
    let mut results: Vec<PatternMatch> = Vec::new();

    for regex in active_patterns {
        for m in regex.find_iter(&text) {
            let start_col = byte_offset_to_col(&text, cells, m.start());
            let end_col = byte_offset_to_col(&text, cells, m.end());
            // 既存マッチと重複していないか確認
            let overlaps =
                results.iter().any(|r| !(end_col <= r.start_col || start_col >= r.end_col));
            if !overlaps && start_col < end_col {
                results.push(PatternMatch { text: m.as_str().to_owned(), start_col, end_col });
            }
        }
    }

    // 列順にソート
    results.sort_by_key(|m| m.start_col);
    results
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

    // -----------------------------------------------------------------------
    // detect_patterns_in_line のテスト
    // -----------------------------------------------------------------------

    #[test]
    fn detect_patterns_finds_file_paths() {
        let cells = make_cells("/usr/local/bin/fish");
        let matches = detect_patterns_in_line(&cells, &[]);
        let path_match = matches.iter().find(|m| m.text == "/usr/local/bin/fish");
        assert!(path_match.is_some(), "ファイルパスが検出されなかった: {matches:?}");
    }

    #[test]
    fn detect_patterns_finds_git_hashes() {
        let cells = make_cells("commit abc1234def5 is the one");
        let matches = detect_patterns_in_line(&cells, &[]);
        let hash_match = matches.iter().find(|m| m.text == "abc1234def5");
        assert!(hash_match.is_some(), "git ハッシュが検出されなかった: {matches:?}");
    }

    #[test]
    fn detect_patterns_finds_urls_and_paths() {
        let cells = make_cells("see https://example.com and /etc/hosts");
        let matches = detect_patterns_in_line(&cells, &[]);
        let url_match = matches.iter().find(|m| m.text == "https://example.com");
        let path_match = matches.iter().find(|m| m.text == "/etc/hosts");
        assert!(url_match.is_some(), "URL が検出されなかった: {matches:?}");
        assert!(path_match.is_some(), "ファイルパスが検出されなかった: {matches:?}");
    }

    #[test]
    fn detect_patterns_no_overlap() {
        // URL にパスが含まれる場合、URL パターンが優先されてパスとして重複検出されないこと
        let cells = make_cells("https://example.com/foo/bar");
        let matches = detect_patterns_in_line(&cells, &[]);
        // start_col が重複するマッチがないこと
        for i in 0..matches.len() {
            for j in (i + 1)..matches.len() {
                let a = &matches[i];
                let b = &matches[j];
                let overlaps = !(a.end_col <= b.start_col || a.start_col >= b.end_col);
                assert!(!overlaps, "マッチが重複している: {:?} と {:?}", a, b);
            }
        }
    }

    #[test]
    fn detect_patterns_empty_cells() {
        let cells = make_cells("");
        let matches = detect_patterns_in_line(&cells, &[]);
        assert!(matches.is_empty());
    }

    #[test]
    fn detect_patterns_custom_patterns() {
        let pattern = Regex::new(r"FOO-\d+").unwrap();
        let cells = make_cells("issue FOO-123 is fixed");
        let matches = detect_patterns_in_line(&cells, &[&pattern]);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].text, "FOO-123");
    }

    #[test]
    fn detect_patterns_sorted_by_col() {
        let cells = make_cells("/var/a and /usr/b");
        let matches = detect_patterns_in_line(&cells, &[]);
        for i in 1..matches.len() {
            assert!(
                matches[i].start_col >= matches[i - 1].start_col,
                "マッチが列順になっていない: {:?}",
                matches
            );
        }
    }

    // -----------------------------------------------------------------------
    // カスタムリンク / テンプレート展開のテスト
    // -----------------------------------------------------------------------

    #[test]
    fn expand_template_dollar_zero() {
        let re = regex::Regex::new(r"JIRA-\d+").unwrap();
        let text = "see JIRA-123 here";
        let caps = re.captures(text).unwrap();
        let result = expand_template("https://jira.example.com/browse/$0", &caps);
        assert_eq!(result, "https://jira.example.com/browse/JIRA-123");
    }

    #[test]
    fn expand_template_capture_groups() {
        let re = regex::Regex::new(r"(\w+)-(\d+)").unwrap();
        let text = "PROJ-456";
        let caps = re.captures(text).unwrap();
        let result = expand_template("https://example.com/$1/issues/$2", &caps);
        assert_eq!(result, "https://example.com/PROJ/issues/456");
    }

    #[test]
    fn expand_template_missing_group_is_empty() {
        let re = regex::Regex::new(r"FOO-(\d+)").unwrap();
        let text = "FOO-789";
        let caps = re.captures(text).unwrap();
        // $2 は存在しないキャプチャグループ → 空文字列
        let result = expand_template("$1-$2", &caps);
        assert_eq!(result, "789-");
    }

    #[test]
    fn extract_url_rejects_javascript_scheme() {
        let re = regex::Regex::new(r"evil").unwrap();
        let caps = re.captures("evil").unwrap();
        let result = extract_url_from_action("open:javascript:alert(1)", &caps);
        assert!(result.is_none(), "javascript: スキームは拒否されるべき");
    }

    #[test]
    fn extract_url_rejects_data_scheme() {
        let re = regex::Regex::new(r"evil").unwrap();
        let caps = re.captures("evil").unwrap();
        let result = extract_url_from_action("open:data:text/html,<h1>XSS</h1>", &caps);
        assert!(result.is_none(), "data: スキームは拒否されるべき");
    }

    #[test]
    fn extract_url_rejects_missing_open_prefix() {
        let re = regex::Regex::new(r"PAT-\d+").unwrap();
        let caps = re.captures("PAT-123").unwrap();
        let result = extract_url_from_action("https://example.com/$0", &caps);
        assert!(result.is_none(), "open: プレフィックスがない場合は None");
    }

    #[test]
    fn extract_url_allows_vscode_scheme() {
        let re = regex::Regex::new(r"file\.rs:\d+").unwrap();
        let caps = re.captures("file.rs:42").unwrap();
        let result = extract_url_from_action("open:vscode://file/$0", &caps);
        assert_eq!(result, Some("vscode://file/file.rs:42".to_owned()));
    }

    #[test]
    fn url_detector_custom_pattern_detected() {
        use crate::config::LinkConfig;
        let links = vec![LinkConfig {
            regex: r"JIRA-\d+".to_owned(),
            action: "open:https://jira.example.com/browse/$0".to_owned(),
        }];
        let detector = UrlDetector::with_links(&links);
        let cells = make_cells("fixed JIRA-123 today");
        // col 6 = 'J' の位置
        let result = detector.find_url_at(&cells, 6);
        assert_eq!(result, Some("https://jira.example.com/browse/JIRA-123".to_owned()));
    }

    #[test]
    fn url_detector_custom_pattern_no_match() {
        use crate::config::LinkConfig;
        let links = vec![LinkConfig {
            regex: r"JIRA-\d+".to_owned(),
            action: "open:https://jira.example.com/browse/$0".to_owned(),
        }];
        let detector = UrlDetector::with_links(&links);
        let cells = make_cells("fixed JIRA-123 today");
        // col 0 = 'f' の位置（マッチ外）
        let result = detector.find_url_at(&cells, 0);
        assert!(result.is_none());
    }

    #[test]
    fn url_detector_invalid_regex_skipped() {
        use crate::config::LinkConfig;
        // 不正な正規表現はスキップされ、パニックしない
        let links = vec![LinkConfig {
            regex: r"[invalid regex".to_owned(),
            action: "open:https://example.com/$0".to_owned(),
        }];
        let detector = UrlDetector::with_links(&links);
        assert_eq!(detector.custom_patterns.len(), 0);
    }
}
