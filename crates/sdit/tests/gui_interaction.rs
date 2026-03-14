//! GUI インタラクションテスト — Layer 3
//!
//! tools/test-utils/ のユーティリティスクリプトを使って SDIT バイナリを操作し、
//! ウィンドウの表示・キー入力・スクリーンショット取得を検証する。
//!
//! # 前提条件
//! - tools/test-utils/ のバイナリをビルド済み（`./tools/test-utils/build.sh`）
//! - Screen Recording 権限を capture-window バイナリに付与済み
//! - 権限付与後に OS を再起動済み
//!
//! # 実行方法
//! ```bash
//! cargo test --test gui_interaction -- --ignored
//! ```
//!
//! すべてのテストに `#[ignore]` が付いているため、通常の `cargo test` では実行されない。

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use std::{fs, thread};

// MARK: - ヘルパー

/// ワークスペースルートのパスを返す。
/// `CARGO_MANIFEST_DIR` は `crates/sdit/` を指すため、2階層上がる。
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root not found")
        .to_path_buf()
}

/// tools/test-utils/ ディレクトリのパスを返す。
fn test_utils_dir() -> PathBuf {
    workspace_root().join("tools").join("test-utils")
}

/// テスト出力の一時ファイル置き場（プロジェクトルートの tmp/）
fn tmp_dir() -> PathBuf {
    workspace_root().join("tmp")
}

/// SDIT バイナリのパス。
/// `CARGO_BIN_EXE_sdit` は cargo が注入する環境変数。
fn sdit_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sdit"))
}

/// window-info バイナリのパス。
fn window_info_bin() -> PathBuf {
    test_utils_dir().join("window-info")
}

/// capture-window バイナリのパス。
fn capture_window_bin() -> PathBuf {
    test_utils_dir().join("capture-window")
}

/// send-keys.sh のパス。
fn send_keys_sh() -> PathBuf {
    test_utils_dir().join("send-keys.sh")
}

/// ユーティリティバイナリが存在するか確認する。
/// 存在しない場合はテストをスキップするためのエラーメッセージを返す。
fn check_utils() -> Result<(), String> {
    let utils = [
        ("window-info", window_info_bin()),
        ("capture-window", capture_window_bin()),
        ("send-keys.sh", send_keys_sh()),
    ];
    for (name, path) in &utils {
        if !path.exists() {
            return Err(format!(
                "ユーティリティ '{name}' が見つかりません: {}\n\
                 tools/test-utils/build.sh を実行してください。",
                path.display()
            ));
        }
    }
    Ok(())
}

/// sdit プロセスが起動してウィンドウが出現するまで待つ（タイムアウト付き）。
/// 成功したら true を返す。
fn wait_for_window(process_name: &str, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        let output = Command::new(window_info_bin())
            .arg(process_name)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output();
        if let Ok(out) = output {
            if out.status.success() {
                return true;
            }
        }
        thread::sleep(Duration::from_millis(200));
    }
    false
}

// MARK: - テスト

/// SDIT を起動してウィンドウが表示されることを確認し、
/// キー入力とスクリーンショット取得を行う。
// smell-allow: silent-skip, conditional-test-logic, sleepy-test — GUI テストは権限・環境依存。プロセス起動待ちに sleep が必要
// smell-allow: redundant-print — #[ignore] 手動実行テスト。eprintln! はオペレーター向け診断出力として意図的に残している
#[test]
#[ignore = "Screen Recording 権限 + GUI 環境が必要。tools/test-utils/build.sh を先に実行すること"]
fn window_appears_and_captures_screenshot() {
    // 最低限のスクリーンショットサイズ閾値: 10 KiB
    const MIN_SCREENSHOT_BYTES: u64 = 10 * 1024;

    // ユーティリティの存在確認
    if let Err(msg) = check_utils() {
        eprintln!("SKIP: {msg}");
        return;
    }

    // tmp/ ディレクトリを確保
    let tmp = tmp_dir();
    fs::create_dir_all(&tmp).expect("tmp/ ディレクトリの作成に失敗");

    // SDIT バイナリを起動（GUI ウィンドウが開く）
    let mut sdit_proc = Command::new(sdit_bin())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("sdit バイナリの起動に失敗");

    // クリーンアップ: テスト終了時に sdit を終了させる
    let _guard = ScopeGuard(Some(Box::new(move || {
        let _ = sdit_proc.kill();
        let _ = sdit_proc.wait();
    }) as Box<dyn FnOnce() + Send>));

    // ウィンドウが出現するまで最大 15 秒待つ
    let appeared = wait_for_window("sdit", Duration::from_secs(15));
    assert!(
        appeared,
        "sdit のウィンドウが 15 秒以内に出現しませんでした。\
         ディスプレイが接続されていることを確認してください。"
    );

    // ウィンドウ属性を取得して JSON をパース
    let info_output =
        Command::new(window_info_bin()).arg("sdit").output().expect("window-info の実行に失敗");
    assert!(
        info_output.status.success(),
        "window-info が失敗: {}",
        String::from_utf8_lossy(&info_output.stderr)
    );

    let json_str = String::from_utf8_lossy(&info_output.stdout);
    assert!(!json_str.is_empty(), "window-info の出力が空です");

    // JSON に必須フィールドが含まれることを確認（簡易チェック）
    assert!(json_str.contains("\"title\""), "JSON に title フィールドがありません: {json_str}");
    assert!(json_str.contains("\"size\""), "JSON に size フィールドがありません: {json_str}");
    assert!(
        json_str.contains("\"position\""),
        "JSON に position フィールドがありません: {json_str}"
    );

    eprintln!("window-info 出力:\n{json_str}");

    // キーストロークを送信（シェルに何か打ち込む）
    let keys_status = Command::new(send_keys_sh())
        .arg("sdit")
        .arg("echo SDIT_GUI_TEST_OK")
        .status()
        .expect("send-keys.sh の実行に失敗");
    assert!(
        keys_status.success(),
        "send-keys.sh が失敗しました。Accessibility 権限を確認してください。"
    );

    // キー入力が反映されるまで少し待つ
    thread::sleep(Duration::from_millis(500));

    // スクリーンショット取得
    let capture_path = tmp.join("test-capture.png");
    let capture_status = Command::new(capture_window_bin())
        .arg("sdit")
        .arg(&capture_path)
        .status()
        .expect("capture-window の実行に失敗");

    // exit 2 は Screen Recording 権限なし
    let code = capture_status.code().unwrap_or(-1);
    assert!(
        code != 2,
        "Screen Recording 権限がありません。\n\
         System Settings → Privacy & Security → Screen Recording で \
         capture-window を許可し、OS を再起動してください。"
    );
    assert!(capture_status.success(), "capture-window が失敗しました (exit {code})");

    // PNG ファイルが存在し、ゼロより大きいことを確認
    assert!(
        capture_path.exists(),
        "スクリーンショットファイルが生成されませんでした: {}",
        capture_path.display()
    );

    let metadata = fs::metadata(&capture_path).expect("ファイルメタデータの取得に失敗");
    let file_size = metadata.len();

    assert!(
        file_size >= MIN_SCREENSHOT_BYTES,
        "スクリーンショットが小さすぎます: {file_size} bytes (期待値: >= {MIN_SCREENSHOT_BYTES} bytes)\n\
         ファイル: {}",
        capture_path.display()
    );

    eprintln!("スクリーンショット取得成功: {} ({file_size} bytes)", capture_path.display());
}

// MARK: - スコープガード（RAII でプロセスをクリーンアップ）

struct ScopeGuard(Option<Box<dyn FnOnce() + Send>>);

impl Drop for ScopeGuard {
    fn drop(&mut self) {
        if let Some(f) = self.0.take() {
            f();
        }
    }
}
