//! ソースファイルの行数制限チェック
//!
//! LLM のコンテキストウィンドウを考慮し、ソースファイルが大きくなりすぎないことを検証する。
//! 閾値を超えたファイルはモジュール分割を検討すること。

use std::path::{Path, PathBuf};
use std::{fs, io};

/// 警告閾値（行）— この行数を超えたら分割を検討
const WARN_THRESHOLD: usize = 1000;
/// エラー閾値（行）— この行数を超えたらテスト失敗
const ERROR_THRESHOLD: usize = 1500;

/// 検査対象のクレートディレクトリ（ワークスペースルートからの相対パス）
const CRATE_DIRS: &[&str] = &[
    "crates/sdit/src",
    "crates/sdit-core/src",
    "crates/sdit-session/src",
    "crates/sdit-render/src",
    "crates/sdit-config/src",
];

fn workspace_root() -> PathBuf {
    // crates/sdit/ から2階層上がワークスペースルート
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root not found")
        .to_path_buf()
}

fn collect_rs_files(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if !dir.exists() {
        return Ok(files);
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            files.extend(collect_rs_files(&path)?);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
    Ok(files)
}

fn count_lines(path: &Path) -> io::Result<usize> {
    let content = fs::read_to_string(path)?;
    Ok(content.lines().count())
}

#[test]
fn source_files_within_line_limits() {
    let root = workspace_root();
    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    for crate_dir in CRATE_DIRS {
        let dir = root.join(crate_dir);
        let files = collect_rs_files(&dir).unwrap_or_default();

        for file in files {
            let Ok(lines) = count_lines(&file) else {
                continue;
            };

            let relative = file.strip_prefix(&root).unwrap_or(&file).display().to_string();

            if lines > ERROR_THRESHOLD {
                errors.push(format!("  {relative}: {lines} 行 (上限 {ERROR_THRESHOLD})"));
            } else if lines > WARN_THRESHOLD {
                warnings.push(format!("  {relative}: {lines} 行 (警告閾値 {WARN_THRESHOLD})"));
            }
        }
    }

    if !warnings.is_empty() {
        eprintln!(
            "\n⚠ 以下のファイルが警告閾値({WARN_THRESHOLD}行)を超えています。分割を検討してください:\n{}",
            warnings.join("\n")
        );
    }

    assert!(
        errors.is_empty(),
        "\n以下のファイルがエラー閾値({ERROR_THRESHOLD}行)を超えています。モジュール分割が必要です:\n{}\n",
        errors.join("\n")
    );
}
