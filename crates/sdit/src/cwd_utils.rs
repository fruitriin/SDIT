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
/// ホスト名は空または `localhost` のみ許可する。
/// パス部分をデコードして `PathBuf` を返す。失敗時は `None`。
pub(crate) fn parse_osc7_cwd(url: &str) -> Option<std::path::PathBuf> {
    // "file://" プレフィックスを除去
    let rest = url.strip_prefix("file://")?;
    // hostname と path を分離: 最初の '/' を探す
    let (host, path_str) = if let Some(slash) = rest.find('/') {
        (&rest[..slash], &rest[slash..]) // "hostname" と "/path"
    } else {
        return None;
    };
    // ホスト名検証: 空または localhost のみ許可
    if !host.is_empty() && !host.eq_ignore_ascii_case("localhost") {
        return None;
    }
    // パーセントエンコードのデコード（簡易版: %XX のみ）
    let decoded = percent_decode(path_str);
    if decoded.is_empty() {
        return None;
    }
    // 制御文字・NUL チェック
    if decoded.bytes().any(|b| b < 0x20 || b == 0x7f) {
        return None;
    }
    let path = std::path::PathBuf::from(&decoded);
    // パストラバーサル防止: ".." コンポーネントを含むパスを拒否
    use std::path::Component;
    if path.components().any(|c| matches!(c, Component::ParentDir)) {
        return None;
    }
    if path.is_absolute() { Some(path) } else { None }
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

    #[test]
    fn parse_osc7_cwd_path_traversal_encoded() {
        // %2F..%2F..%2Fetc%2Fpasswd のようなエンコードされたパストラバーサルを拒否
        assert!(parse_osc7_cwd("file:///..%2F..%2Fetc%2Fpasswd").is_none());
    }

    #[test]
    fn parse_osc7_cwd_path_traversal_literal() {
        // リテラルな ".." コンポーネントを含むパスを拒否
        assert!(parse_osc7_cwd("file:///home/user/../../etc/passwd").is_none());
    }

    #[test]
    fn parse_osc7_cwd_remote_host_rejected() {
        // リモートホスト名は拒否
        assert!(parse_osc7_cwd("file://remotehost/home/user").is_none());
        assert!(parse_osc7_cwd("file://evil.example.com/etc/passwd").is_none());
    }

    #[test]
    fn parse_osc7_cwd_control_char_rejected() {
        // 制御文字（NUL バイト等）を含むパスを拒否
        // %00 = NUL, %01 = SOH
        assert!(parse_osc7_cwd("file:///home/user%00/projects").is_none());
        assert!(parse_osc7_cwd("file:///home/user%01/projects").is_none());
    }
}
