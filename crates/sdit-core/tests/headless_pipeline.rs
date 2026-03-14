//! Headless integration test: PTY → VTE → Grid pipeline.
//!
//! Spawns a real PTY child process, reads its output, feeds it through the VTE
//! parser into a Terminal, and verifies the resulting grid state.

#![allow(clippy::cast_possible_wrap)]

use std::io::{Read, Write};
use std::time::{Duration, Instant};

use sdit_core::grid::Dimensions;
use sdit_core::index::{Column, Line, Point};
use sdit_core::pty::{Pty, PtyConfig, PtySize};
use sdit_core::terminal::{Processor, Terminal};

/// Helper: check if we have a working TTY (CI may not).
fn is_tty() -> bool {
    std::fs::OpenOptions::new().read(true).write(true).open("/dev/tty").is_ok()
}

/// Read from PTY until `predicate` returns true or timeout.
fn read_until(
    pty: &mut Pty,
    terminal: &mut Terminal,
    processor: &mut Processor,
    timeout: Duration,
    predicate: impl Fn(&Terminal) -> bool,
) {
    let mut buf = [0u8; 4096];
    let deadline = Instant::now() + timeout;

    loop {
        if predicate(terminal) {
            return;
        }
        if Instant::now() >= deadline {
            return;
        }
        match pty.read(&mut buf) {
            Ok(0) => return,
            Ok(n) => processor.advance(terminal, &buf[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(5));
            }
            // EIO: PTY closed (child exited)
            Err(e) if e.raw_os_error() == Some(5) => return,
            Err(_) => return,
        }
    }
}

/// Collect the text content of a grid row (trimming trailing spaces).
fn row_text(terminal: &Terminal, line: i32) -> String {
    let cols = terminal.grid().columns();
    let mut s = String::new();
    for col in 0..cols {
        s.push(terminal.grid()[Point::new(Line(line), Column(col))].c);
    }
    s.trim_end().to_string()
}

// smell-allow: silent-skip, conditional-test-logic — TTY がない CI では PTY 統合テスト不可
#[test]
fn echo_appears_in_grid() {
    if !is_tty() {
        return;
    }

    let size = PtySize::new(24, 80);
    let config = PtyConfig {
        shell: Some("echo".to_owned()),
        args: vec!["SDIT_TEST_OUTPUT".to_owned()],
        working_directory: None,
        env: std::collections::HashMap::new(),
    };

    let mut pty = Pty::spawn(&config, size).expect("spawn failed");
    let mut terminal = Terminal::new(24, 80, 1000);
    let mut processor = Processor::new();

    // Read until we see the test string in the grid.
    read_until(&mut pty, &mut terminal, &mut processor, Duration::from_secs(5), |term| {
        (0..term.grid().screen_lines() as i32)
            .any(|line| row_text(term, line).contains("SDIT_TEST_OUTPUT"))
    });

    // Verify at least one row contains our marker.
    let found = (0..terminal.grid().screen_lines() as i32)
        .any(|line| row_text(&terminal, line).contains("SDIT_TEST_OUTPUT"));
    assert!(found, "expected 'SDIT_TEST_OUTPUT' in grid");

    let _ = pty.try_wait();
}

// smell-allow: silent-skip, conditional-test-logic — TTY がない CI では PTY 統合テスト不可
#[test]
fn shell_command_pipeline() {
    if !is_tty() {
        return;
    }

    let size = PtySize::new(24, 80);
    let config = PtyConfig {
        shell: Some("/bin/sh".to_owned()),
        args: vec![],
        working_directory: None,
        env: std::collections::HashMap::new(),
    };

    let mut pty = Pty::spawn(&config, size).expect("spawn failed");
    let mut terminal = Terminal::new(24, 80, 1000);
    let mut processor = Processor::new();

    // シェル初期化を待つ: 初期出力（プロンプト等）をポーリングで読み切る
    read_until(&mut pty, &mut terminal, &mut processor, Duration::from_secs(2), |_| false);

    // Send a command that produces known output.
    pty.write_all(b"printf 'GRID_CHECK_42\\n'\n").expect("write failed");

    // Read until the output appears.
    read_until(&mut pty, &mut terminal, &mut processor, Duration::from_secs(5), |term| {
        (0..term.grid().screen_lines() as i32)
            .any(|line| row_text(term, line).contains("GRID_CHECK_42"))
    });

    let found = (0..terminal.grid().screen_lines() as i32)
        .any(|line| row_text(&terminal, line).contains("GRID_CHECK_42"));
    assert!(found, "expected 'GRID_CHECK_42' in grid after shell command");

    let _ = pty.kill();
}

/// macOS 26 PTY ioctl 互換性の退行テスト。
///
/// macOS 26 では spawn 前に TIOCSWINSZ を呼ぶと ENOTTY になる。
/// `Pty::spawn()` の実装が「spawn 後に resize」の順序を守っていることを検証する。
// smell-allow: silent-skip, conditional-test-logic — TTY がない CI では PTY 統合テスト不可
#[test]
fn pty_spawn_then_resize() {
    if !is_tty() {
        return;
    }

    let initial_size = PtySize::new(24, 80);
    let config = PtyConfig {
        shell: Some("/bin/sh".to_owned()),
        args: vec![],
        working_directory: None,
        env: std::collections::HashMap::new(),
    };

    let mut pty = Pty::spawn(&config, initial_size).expect("spawn failed");

    // spawn 後の resize は成功すべき（macOS 26 PTY 互換性の核心部分）。
    let new_size = PtySize::new(30, 100);
    pty.resize(new_size)
        .expect("resize after spawn should succeed on all platforms including macOS 26");

    // resize 後も PTY は正常に動作していることを確認する。
    let status = pty.try_wait().expect("try_wait after resize should not error");
    // シェルはまだ生きているはずなので None
    assert!(status.is_none(), "shell should still be running after resize");

    let _ = pty.kill();
}

// smell-allow: silent-skip, conditional-test-logic — TTY がない CI では PTY 統合テスト不可
#[test]
fn cursor_position_after_escape_sequence() {
    if !is_tty() {
        return;
    }

    let size = PtySize::new(24, 80);
    let config = PtyConfig {
        shell: Some("/bin/sh".to_owned()),
        args: vec!["-c".to_owned(), "printf '\\033[5;10Hmarker'".to_owned()],
        working_directory: None,
        env: std::collections::HashMap::new(),
    };

    let mut pty = Pty::spawn(&config, size).expect("spawn failed");
    let mut terminal = Terminal::new(24, 80, 1000);
    let mut processor = Processor::new();

    read_until(&mut pty, &mut terminal, &mut processor, Duration::from_secs(5), |term| {
        // Row 4 (0-indexed), column 9 (0-indexed) should have 'm' (start of "marker")
        term.grid()[Point::new(Line(4), Column(9))].c == 'm'
    });

    // Verify "marker" at row 4, col 9.
    let text = row_text(&terminal, 4);
    assert!(text.contains("marker"), "expected 'marker' at row 4, got: {text:?}");

    let _ = pty.try_wait();
}
