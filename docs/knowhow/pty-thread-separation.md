# PTY read/write スレッド分離パターン

## 問題

`pty_process::blocking::Pty` はブロッキング I/O。1スレッドで read + write を処理すると、`read()` がブロックしている間 `try_recv()` に到達せず、キー入力が PTY に届かないデッドロックが発生する。

## 解決策

PTY の master fd をクローンして read/write を独立スレッドに分離する。

```rust
// sdit-core/pty/mod.rs
pub fn try_clone_writer(&self) -> Result<std::fs::File> {
    use std::os::fd::AsFd;
    let fd = self.pty.as_fd().try_clone_to_owned().map_err(PtyError::Io)?;
    Ok(std::fs::File::from(fd))
}
```

### スレッド構成

| スレッド | 所有物 | 動作 |
|---|---|---|
| pty-reader | `Pty` (元の fd) | `pty.read()` → Terminal 更新 → PtyOutput イベント |
| pty-writer | `File` (クローン fd) | `pty_write_rx.recv()` → `writer.write_all()` |
| GUI (main) | `pty_write_tx` | キー入力 → `tx.try_send(bytes)` |

### ポイント

- `try_clone_to_owned()` は POSIX `dup(2)` 相当。別の fd 番号が割り当てられるためダブルクローズなし
- `unsafe` 不要（`AsFd` + `OwnedFd` の標準 API のみ使用）
- writer スレッドは `recv()` でブロッキング待機するため CPU を消費しない
- reader スレッドはブロッキング read に専念でき、コードがシンプルに
- `pty_process::blocking::Pty` は `Read`/`Write` を `&Pty` にも実装しているが、`Arc` でラップするより fd クローンの方がロック不要で高性能

## 前提

- `pty-process = "0.4"` が `AsFd` を実装していること
- `unsafe_code = "deny"` 環境でも動作すること
