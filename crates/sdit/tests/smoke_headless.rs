//! Headless smoke test: `--headless` モードがエンドツーエンドで正常動作することを確認する。
//!
//! CI 環境（TTY なし）でも実行可能。winit/wgpu を一切使わない。

use std::process::Command;
use std::time::{Duration, Instant};

/// バイナリパスを取得する。
fn sdit_bin() -> String {
    env!("CARGO_BIN_EXE_sdit").to_string()
}

/// 子プロセスを spawn してタイムアウト付きで待機する。
///
/// タイムアウト前に終了した場合は `Some(ExitStatus)`、タイムアウトした場合は `None`。
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
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => return None,
        }
    }
}

#[test]
fn headless_mode_exits_successfully() {
    let mut child = Command::new(sdit_bin())
        .arg("--headless")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn sdit --headless");

    let Some(status) = wait_with_timeout(&mut child, Duration::from_secs(10)) else {
        // タイムアウト: 強制終了してテスト失敗
        let _ = child.kill();
        let _ = child.wait();
        panic!("sdit --headless timed out after 10 seconds");
    };

    // stderr を収集してパニック・エラーの有無を確認する。
    let output = child.wait_with_output();
    let stderr = output
        .as_ref()
        .map(|o| String::from_utf8_lossy(&o.stderr).into_owned())
        .unwrap_or_default();

    assert!(!stderr.contains("panic"), "headless mode panicked:\n{stderr}");

    assert!(
        status.success(),
        "headless mode exited with non-zero status: {status:?}\nstderr:\n{stderr}"
    );
}

#[test]
fn headless_mode_stderr_no_error_keywords() {
    let output = Command::new(sdit_bin())
        .arg("--headless")
        .env("RUST_LOG", "info")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            // タイムアウト付き wait
            let deadline = Instant::now() + Duration::from_secs(10);
            loop {
                match child.try_wait() {
                    Ok(Some(_)) | Err(_) => break,
                    Ok(None) => {
                        if Instant::now() >= deadline {
                            let _ = child.kill();
                        }
                    }
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            child.wait_with_output()
        })
        .expect("failed to run sdit --headless");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // パニックスタックトレースやクリティカルエラーがないことを確認する。
    assert!(!stderr.contains("SIGSEGV"), "headless mode caused SIGSEGV:\n{stderr}");
    assert!(!stderr.contains("SIGABRT"), "headless mode caused SIGABRT:\n{stderr}");
    assert!(!stderr.contains("thread '"), "headless mode thread panicked:\n{stderr}");

    // 期待されるログメッセージが出力されていることを確認する。
    assert!(
        stderr.contains("SDIT starting in headless mode"),
        "headless mode should log startup message.\nstderr:\n{stderr}"
    );
}
