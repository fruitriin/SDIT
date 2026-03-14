//! CWD 関連のユーティリティ関数とクリップボードヘルパー。

/// クリップボードコピー用: 各行末の空白を削除する。
///
/// `selection.trim_trailing_spaces = true` のとき呼ばれる。
pub(crate) fn trim_trailing_whitespace(text: &str) -> String {
    text.lines().map(str::trim_end).collect::<Vec<_>>().join("\n")
}

/// OSC 7 の `file://hostname/path` URL から `PathBuf` に変換する。
///
/// `file://hostname/path` または `file:///path` 形式を受け付ける。
/// パス部分をデコードして `PathBuf` を返す。失敗時は `None`。
pub(crate) fn parse_osc7_cwd(url: &str) -> Option<std::path::PathBuf> {
    // "file://" プレフィックスを除去
    let rest = url.strip_prefix("file://")?;
    // hostname を除去: 最初の '/' を探す
    let path_str = if let Some(slash) = rest.find('/') {
        &rest[slash..] // "/path" 部分
    } else {
        return None;
    };
    // パーセントエンコードのデコード（簡易版: %XX のみ）
    let decoded = percent_decode(path_str);
    if decoded.is_empty() {
        return None;
    }
    Some(std::path::PathBuf::from(decoded))
}

/// パーセントエンコードされた文字列をデコードする（パス用簡易版）。
fn percent_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) =
                (char::from(bytes[i + 1]).to_digit(16), char::from(bytes[i + 2]).to_digit(16))
            {
                let byte = ((h * 16) + l) as u8;
                out.push(char::from(byte));
                i += 3;
                continue;
            }
        }
        out.push(char::from(bytes[i]));
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trim_trailing_whitespace_removes_spaces() {
        assert_eq!(trim_trailing_whitespace("hello   \nworld  "), "hello\nworld");
    }

    #[test]
    fn trim_trailing_whitespace_preserves_content() {
        assert_eq!(trim_trailing_whitespace("hello\nworld"), "hello\nworld");
    }

    #[test]
    fn trim_trailing_whitespace_empty() {
        assert_eq!(trim_trailing_whitespace(""), "");
    }

    #[test]
    fn trim_trailing_whitespace_only_spaces() {
        assert_eq!(trim_trailing_whitespace("   "), "");
    }

    #[test]
    fn parse_osc7_cwd_simple() {
        let path = parse_osc7_cwd("file:///home/user").unwrap();
        assert_eq!(path, std::path::PathBuf::from("/home/user"));
    }

    #[test]
    fn parse_osc7_cwd_with_hostname() {
        let path = parse_osc7_cwd("file://localhost/home/user/projects").unwrap();
        assert_eq!(path, std::path::PathBuf::from("/home/user/projects"));
    }

    #[test]
    fn parse_osc7_cwd_invalid_scheme() {
        assert!(parse_osc7_cwd("http://example.com/path").is_none());
    }

    #[test]
    fn parse_osc7_cwd_no_path() {
        assert!(parse_osc7_cwd("file://localhost").is_none());
    }
}
