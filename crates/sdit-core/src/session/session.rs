use std::os::fd::OwnedFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use crate::pty::{Pty, PtyConfig, PtySize};
use crate::terminal::{Processor, Terminal};

/// セッション ID（単調増加）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SessionId(pub u64);

/// Terminal + Processor をまとめてロックで保護する。
pub struct TerminalState {
    pub terminal: Terminal,
    pub processor: Processor,
}

/// PTY I/O スレッドのハンドル群。
pub struct PtyIo {
    /// PTY への書き込みチャネル（GUI → Writer スレッド）。
    pub write_tx: mpsc::SyncSender<Vec<u8>>,
    /// Reader スレッドハンドル。
    reader: Option<JoinHandle<()>>,
    /// Writer スレッドハンドル。
    writer: Option<JoinHandle<()>>,
}

/// 1つの PTY セッション。
///
/// Session は Window に依存しない。Window の生成・破棄とは独立して生存し、
/// Phase 4 の合体・切出し時は Window 側の参照先を差し替えるだけで PTY は切れない。
///
/// Drop 時に子プロセスへ SIGHUP を送信し、I/O スレッドの終了を待つ。
pub struct Session {
    pub id: SessionId,
    pub term_state: Arc<Mutex<TerminalState>>,
    pub pty_io: PtyIo,
    /// ユーザーが設定したカスタムセッション名。`None` の場合はデフォルト名を表示する。
    pub custom_name: Option<String>,
    /// OSC 7 で通知された最新のカレントディレクトリ。
    pub cwd: Option<PathBuf>,
    /// PTY master fd のクローン（リサイズ ioctl 専用）。
    resize_fd: OwnedFd,
    /// 子プロセスの PID（Drop 時のシグナル送信用）。
    child_pid: u32,
    /// 子プロセスが既に終了しているかのフラグ。
    /// PID 再利用によるシグナル誤送信を防ぐ。
    child_exited: Arc<AtomicBool>,
}

/// Session 生成用パラメータ。
pub struct SpawnParams<F> {
    pub pty_config: PtyConfig,
    pub pty_size: PtySize,
    pub terminal_rows: usize,
    pub terminal_cols: usize,
    pub scrollback: usize,
    /// デフォルトカーソルスタイル（設定ファイルから）。
    pub default_cursor_style: crate::terminal::CursorStyle,
    /// デフォルトカーソル点滅（設定ファイルから）。
    pub default_cursor_blinking: bool,
    /// PTY Reader/Writer スレッドを生成するファクトリ。
    /// `(Pty, Arc<Mutex<TerminalState>>, Arc<AtomicBool>)` を受け取り
    /// `(reader, writer, write_tx)` を返す。
    /// 3つ目の `AtomicBool` は子プロセス終了時に `true` に設定すること。
    pub spawn_reader: F,
}

impl Session {
    /// 新しいセッションを生成する。
    ///
    /// PTY を起動し、Reader/Writer スレッドを立ち上げる。
    /// Reader スレッドの生成は呼び出し側がカスタムできるよう `spawn_reader` で受け取る
    /// （winit の `EventLoopProxy` への依存を session から排除するため）。
    pub fn spawn<F>(id: SessionId, params: SpawnParams<F>) -> crate::pty::Result<Self>
    where
        F: FnOnce(
            Pty,
            Arc<Mutex<TerminalState>>,
            Arc<AtomicBool>,
        ) -> (JoinHandle<()>, JoinHandle<()>, mpsc::SyncSender<Vec<u8>>),
    {
        let terminal = Terminal::new_with_cursor(
            params.terminal_rows,
            params.terminal_cols,
            params.scrollback,
            params.default_cursor_style,
            params.default_cursor_blinking,
        );
        let processor = Processor::new();
        let term_state = Arc::new(Mutex::new(TerminalState { terminal, processor }));

        let pty = Pty::spawn(&params.pty_config, params.pty_size)?;

        // Pty を Reader スレッドに move する前に、リサイズ用 fd と子プロセス PID を取得する。
        let resize_fd = pty.try_clone_resize_fd()?;
        let child_pid = pty.child_id();
        let child_exited = Arc::new(AtomicBool::new(false));

        let (reader_handle, writer_handle, write_tx) =
            (params.spawn_reader)(pty, Arc::clone(&term_state), Arc::clone(&child_exited));

        Ok(Self {
            id,
            term_state,
            pty_io: PtyIo { write_tx, reader: Some(reader_handle), writer: Some(writer_handle) },
            custom_name: None,
            cwd: None,
            resize_fd,
            child_pid,
            child_exited,
        })
    }

    /// シェルがサブプロセスを持っているかどうかを返す（フォアグラウンドプロセス実行中の判定）。
    ///
    /// `pgrep -P <pid>` コマンドで子プロセスの有無を確認する。
    /// `pgrep` が存在しない環境では `false` を返す（安全側に倒す = 確認なしで閉じる）。
    /// 子プロセスが既に終了している場合も `false` を返す。
    pub fn has_foreground_process(&self) -> bool {
        // 子プロセスが既に終了していれば false
        if self.child_exited.load(Ordering::Acquire) {
            return false;
        }
        std::process::Command::new("pgrep")
            .arg("-P")
            .arg(self.child_pid.to_string())
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// PTY のターミナルサイズを変更する（SIGWINCH を子プロセスに送信する）。
    pub fn resize_pty(&self, size: PtySize) {
        use std::os::fd::AsFd;
        let winsize = rustix::termios::Winsize {
            ws_row: size.rows,
            ws_col: size.cols,
            ws_xpixel: size.pixel_width,
            ws_ypixel: size.pixel_height,
        };
        if let Err(e) = rustix::termios::tcsetwinsize(self.resize_fd.as_fd(), winsize) {
            log::warn!("PTY resize failed (session {}): {e}", self.id.0);
        }
    }
}

/// PID を `rustix::process::Pid` に安全に変換する。
/// 変換に失敗した場合（PID 0 や i32 に収まらない値）は `None` を返す。
fn to_rustix_pid(pid: u32) -> Option<rustix::process::Pid> {
    let raw: i32 = pid.try_into().ok()?;
    rustix::process::Pid::from_raw(raw)
}

/// スレッドの終了を deadline まで待ち、終了していれば join する。
fn wait_and_join(handle: JoinHandle<()>, deadline: std::time::Instant, label: &str, sid: u64) {
    while !handle.is_finished() && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    if handle.is_finished() {
        let _ = handle.join();
    } else {
        log::warn!("Session {sid}: {label} thread did not finish in time");
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // 1. 子プロセスが既に終了していなければ SIGHUP を送信する。
        //    PID 再利用による誤送信を防ぐため child_exited フラグをチェックする。
        let should_signal = !self.child_exited.load(Ordering::Acquire);
        if should_signal {
            if let Some(pid) = to_rustix_pid(self.child_pid) {
                let _ = rustix::process::kill_process(pid, rustix::process::Signal::Hup);
            }
        }

        // 2. Writer スレッドを先に join（write_tx drop で即座に終了するはず）。
        //    Writer には 200ms、Reader には 300ms を割り当てる。
        let writer_deadline = std::time::Instant::now() + Duration::from_millis(200);
        if let Some(writer) = self.pty_io.writer.take() {
            wait_and_join(writer, writer_deadline, "writer", self.id.0);
        }

        // 3. Reader スレッドを join（SIGHUP で read() が EIO を返して終了するはず）。
        let reader_deadline = std::time::Instant::now() + Duration::from_millis(300);
        if let Some(reader) = self.pty_io.reader.take() {
            // Reader がまだ終了しない場合は SIGKILL で強制終了する。
            if !reader.is_finished() && should_signal {
                if let Some(pid) = to_rustix_pid(self.child_pid) {
                    let _ = rustix::process::kill_process(pid, rustix::process::Signal::Kill);
                }
            }
            wait_and_join(reader, reader_deadline, "reader", self.id.0);
        }
    }
}
