//! GUI smoke test: `SDIT_SMOKE_TEST=1` でアプリが起動し、1フレーム描画後に正常終了することを確認する。
//!
//! このテストはディスプレイ環境（macOS のウィンドウサーバー）が必要なため、
//! デフォルトでは `#[ignore]` にしている。
//!
//! 手動実行:
//! ```sh
//! cargo test -p sdit --test smoke_gui -- --ignored
//! ```

use std::process::Command;
use std::time::{Duration, Instant};

/// バイナリパスを取得する。
fn sdit_bin() -> String {
    env!("CARGO_BIN_EXE_sdit").to_string()
}

/// 子プロセスをタイムアウト付きで待機する。
///
/// タイムアウト前に終了した場合は `Some(status)`、タイムアウトした場合は `None`。
fn wait_with_timeout(
    child: &mut std::process::Child,
    timeout: Duration,
) -> Option<std::process::ExitStatus> {
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) => {
                if Instant::now() >= deadline {
                    return None;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(_) => return None,
        }
    }
}

#[test]
#[ignore = "ディスプレイ環境（macOS ウィンドウサーバー）が必要"]
fn gui_smoke_test_exits_successfully() {
    let mut child = Command::new(sdit_bin())
        .env("SDIT_SMOKE_TEST", "1")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn sdit with SDIT_SMOKE_TEST=1");

    let Some(status) = wait_with_timeout(&mut child, Duration::from_secs(15)) else {
        // タイムアウト: 強制終了してテスト失敗
        let _ = child.kill();
        let _ = child.wait();
        panic!("sdit SDIT_SMOKE_TEST=1 timed out after 15 seconds");
    };

    // stderr を収集してパニック・エラーの有無を確認する。
    let output = child.wait_with_output();
    let stderr = output
        .as_ref()
        .map(|o| String::from_utf8_lossy(&o.stderr).into_owned())
        .unwrap_or_default();

    assert!(!stderr.contains("panic"), "GUI smoke test panicked:\n{stderr}");

    assert!(!stderr.contains("thread '"), "GUI smoke test caused thread panic:\n{stderr}");

    assert!(
        status.success(),
        "GUI smoke test exited with non-zero status: {status:?}\nstderr:\n{stderr}"
    );
}
