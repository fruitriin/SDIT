# macOS 26 PTY 互換性の知見

## TIOCSWINSZ ioctl の挙動変更

macOS 26 (Darwin 25.3.0) で `pty-process` 0.4 の `Pty::resize()` が
spawn 前に呼ぶと `ENOTTY (errno 25)` を返すようになった。

### 症状

```
pty-process error: Inappropriate ioctl for device (os error 25)
```

### 原因

`pty_process::blocking::Pty::new()` で作成した直後の PTY master fd に対して
`TIOCSWINSZ` ioctl を発行すると macOS 26 では ENOTTY になる。
macOS 15 以前では同じ操作が成功していた。

### 解決策

`cmd.spawn(&pts)` の後に `pty.resize()` を呼ぶ。
spawn 後であれば PTY master fd への ioctl は正常に動作する。

```rust
// NG: macOS 26 で ENOTTY
let pty = Pty::new()?;
pty.resize(Size::new(24, 80))?;  // ENOTTY!
let pts = pty.pts()?;
let child = cmd.spawn(&pts)?;

// OK: spawn 後なら成功
let pty = Pty::new()?;
let pts = pty.pts()?;
let child = cmd.spawn(&pts)?;
pty.resize(Size::new(24, 80))?;  // OK
```

### 影響範囲

- `sdit-core/src/pty/mod.rs` の `Pty::spawn()` を修正済み
- PTY テストの `is_tty()` チェック: `/dev/tty` が開けなくても `Pty::new()` 自体は成功する
  - Claude Code のサンドボックス環境でも spawn 自体は可能（要検証）
- pty-process 0.4 固有の問題か OS 側の変更かは未確定
  - 他のターミナルエミュレータ（Alacritty 等）も同様の影響を受ける可能性あり

### 検証方法

```bash
cargo run -p sdit-core --example debug_pty
```
（デバッグ用 example は削除済み。必要なら再作成する）
