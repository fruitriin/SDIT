use std::io::Read;

use sdit_core::grid::Dimensions;
use sdit_core::pty::{Pty, PtyConfig, PtySize};
use sdit_core::terminal::Terminal;

pub(crate) fn grid_contains(terminal: &Terminal, needle: &str) -> bool {
    use sdit_core::index::{Column, Line, Point};
    let rows = terminal.grid().screen_lines();
    let cols = terminal.grid().columns();
    (0..rows).any(|r| {
        #[allow(clippy::cast_possible_wrap)]
        let line = Line(r as i32);
        let mut row = String::new();
        for c in 0..cols {
            row.push(terminal.grid()[Point::new(line, Column(c))].c);
        }
        row.contains(needle)
    })
}

pub(crate) fn run_headless() -> ! {
    use sdit_core::terminal::Processor;

    let size = PtySize::new(24, 80);
    let config = PtyConfig {
        shell: Some("/bin/sh".to_owned()),
        args: vec!["-c".to_owned(), "echo SDIT_HEADLESS_OK".to_owned()],
        working_directory: None,
        env: std::collections::HashMap::new(),
    };

    let mut pty = match Pty::spawn(&config, size) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("headless: PTY spawn failed: {e}");
            std::process::exit(1);
        }
    };

    let mut terminal = Terminal::new(24, 80, 1000);
    let mut processor = Processor::new();

    let timeout = std::time::Duration::from_secs(5);
    let deadline = std::time::Instant::now() + timeout;
    let mut buf = [0u8; 4096];

    loop {
        if grid_contains(&terminal, "SDIT_HEADLESS_OK") {
            std::process::exit(0);
        }

        if std::time::Instant::now() >= deadline {
            eprintln!("headless: timeout waiting for SDIT_HEADLESS_OK");
            std::process::exit(1);
        }

        match pty.read(&mut buf) {
            Ok(0) => {
                if grid_contains(&terminal, "SDIT_HEADLESS_OK") {
                    std::process::exit(0);
                } else {
                    eprintln!("headless: EOF reached without finding SDIT_HEADLESS_OK");
                    std::process::exit(1);
                }
            }
            Ok(n) => {
                processor.advance(&mut terminal, &buf[..n]);
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            Err(e) if e.raw_os_error() == Some(5) => {
                if grid_contains(&terminal, "SDIT_HEADLESS_OK") {
                    std::process::exit(0);
                } else {
                    eprintln!("headless: EIO without finding SDIT_HEADLESS_OK");
                    std::process::exit(1);
                }
            }
            Err(e) => {
                eprintln!("headless: PTY read error: {e}");
                std::process::exit(1);
            }
        }
    }
}
