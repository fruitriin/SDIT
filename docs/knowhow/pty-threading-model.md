# PTY スレッドモデルの知見

## 2スレッドモデル（Alacritty 参照）

```
Main Thread (winit event loop)
├── Window events → キー入力バイト → mpsc channel
├── SditEvent::PtyOutput → terminal.lock() → GPU描画
└── SditEvent::ChildExit → event_loop.exit()

PTY Reader Thread
├── mpsc try_recv → pty.write_all(data)  // 書き込み
├── pty.read(buf) → processor.advance()  // 読み取り + VTE parse
└── event_proxy.send_event(PtyOutput)    // 再描画要求
```

## Terminal 状態共有

`Arc<Mutex<TerminalState>>` で Terminal + Processor を一緒に保護。
Processor は `&mut Terminal` を必要とするため分離不可。

## PTY I/O の統合

PTY reader スレッドが Pty を所有し、書き込み要求は `mpsc::sync_channel` で受け取る。
`sync_channel(64)` で容量制限し、DoS 対策。

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
- PTY リサイズ時の SIGWINCH（現在は Terminal::resize() のみ）
