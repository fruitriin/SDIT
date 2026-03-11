# PTY スレッドモデルの知見

## 3スレッドモデル（現行）

```
Main Thread (winit event loop)
├── Window events → キー入力バイト → mpsc channel → PTY Writer
├── SditEvent::PtyOutput → terminal.lock() → GPU描画
└── SditEvent::ChildExit → event_loop.exit()

PTY Reader Thread (owns Pty)
├── pty.read(buf) → processor.advance()  // 読み取り + VTE parse
└── event_proxy.send_event(PtyOutput)    // 再描画要求

PTY Writer Thread (owns cloned fd as File)
├── pty_write_rx.recv() → writer.write_all(data)  // ブロッキング待機 + 書き込み
└── write error → event_proxy.send_event(ChildExit)
```

### 旧モデルの問題（デッドロック）

旧実装は PTY Reader 1スレッドで read + write を処理していた:
```
// 旧: デッドロックする構造
loop {
    while let Ok(data) = pty_write_rx.try_recv() { pty.write_all(&data); }  // ← read がブロック中は到達しない
    pty.read(&mut buf);  // ← ブロッキング
}
```

`pty_process::blocking::Pty` の read はブロッキングのため、シェルが入力待ちの間 `try_recv()` に到達せず、キー入力が永遠に PTY に届かなかった。

### fd クローンによる分離

```rust
// sdit-core/pty/mod.rs
pub fn try_clone_writer(&self) -> Result<std::fs::File> {
    use std::os::fd::AsFd;
    let fd = self.pty.as_fd().try_clone_to_owned().map_err(PtyError::Io)?;
    Ok(std::fs::File::from(fd))
}
```

- `try_clone_to_owned()` は POSIX `dup(2)` 相当。別の fd 番号が割り当てられるためダブルクローズなし
- `unsafe` 不要（`AsFd` + `OwnedFd` の標準 API のみ）
- `pty-process = "0.4"` が `AsFd` を実装していることが前提

## Terminal 状態共有

`Arc<Mutex<TerminalState>>` で Terminal + Processor を一緒に保護。
Processor は `&mut Terminal` を必要とするため分離不可。

## PTY I/O チャネル設計

`mpsc::sync_channel(64)` で容量制限し、DoS 対策。
`try_send` が `Full` を返した場合は `log::warn!` でログ出力。
Writer スレッドは `recv()` でブロッキング待機するため CPU を消費しない。

## ブロッキング read の扱い

- `WouldBlock` → 1ms sleep（busy-wait 回避）
- `EIO (errno 5)` → PTY closed（子プロセス終了）
- EOF (read 0) → break

## Mutex Poisoning

`unwrap_or_else(PoisonError::into_inner)` で継続動作。
Phase 3 以降でリセット or 安全終了を検討。

## キー入力変換

- `winit::keyboard::Key::Character(s)` → UTF-8 バイト列
- Ctrl+a-z → 0x01-0x1a（コントロールコード）
- Arrow キー → `TermMode::APP_CURSOR` に応じて `\x1bOX` or `\x1b[X`
- Enter → `\r`, Backspace → `\x7f`, Tab → `\t`, Escape → `\x1b`

## 要改善事項

- PtyOutput イベントのバッチング（高速出力時のイベント連射対策）
- Ctrl+記号キーの完全マッピング（`[` → ESC, `\` → 0x1c 等）
- PTY リサイズ時の SIGWINCH（現在は Terminal::resize() のみ、Pty::resize() 未呼び出し）
