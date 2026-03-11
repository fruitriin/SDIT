use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use sdit_core::pty::{Pty, PtyConfig, PtySize};
use sdit_core::terminal::{Processor, Terminal};

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
    /// Reader スレッドハンドル（Drop で join はしない）。
    _reader: JoinHandle<()>,
    /// Writer スレッドハンドル。
    _writer: JoinHandle<()>,
}

/// 1つの PTY セッション。
///
/// Session は Window に依存しない。Window の生成・破棄とは独立して生存し、
/// Phase 4 の合体・切出し時は Window 側の参照先を差し替えるだけで PTY は切れない。
pub struct Session {
    pub id: SessionId,
    pub term_state: Arc<Mutex<TerminalState>>,
    pub pty_io: PtyIo,
}

/// Session 生成用パラメータ。
pub struct SpawnParams<F> {
    pub pty_config: PtyConfig,
    pub pty_size: PtySize,
    pub terminal_rows: usize,
    pub terminal_cols: usize,
    pub scrollback: usize,
    /// PTY Reader スレッドを生成するファクトリ。
    /// `(Pty, Arc<Mutex<TerminalState>>)` を受け取り `JoinHandle` を返す。
    pub spawn_reader: F,
}

impl Session {
    /// 新しいセッションを生成する。
    ///
    /// PTY を起動し、Reader/Writer スレッドを立ち上げる。
    /// Reader スレッドの生成は呼び出し側がカスタムできるよう `spawn_reader` で受け取る
    /// （winit の `EventLoopProxy` への依存を sdit-session から排除するため）。
    pub fn spawn<F>(id: SessionId, params: SpawnParams<F>) -> sdit_core::pty::Result<Self>
    where
        F: FnOnce(
            Pty,
            Arc<Mutex<TerminalState>>,
        ) -> (JoinHandle<()>, JoinHandle<()>, mpsc::SyncSender<Vec<u8>>),
    {
        let terminal = Terminal::new(params.terminal_rows, params.terminal_cols, params.scrollback);
        let processor = Processor::new();
        let term_state = Arc::new(Mutex::new(TerminalState { terminal, processor }));

        let pty = Pty::spawn(&params.pty_config, params.pty_size)?;

        let (reader_handle, writer_handle, write_tx) =
            (params.spawn_reader)(pty, Arc::clone(&term_state));

        Ok(Self {
            id,
            term_state,
            pty_io: PtyIo { write_tx, _reader: reader_handle, _writer: writer_handle },
        })
    }
}
