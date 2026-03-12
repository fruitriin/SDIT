# ターミナルデバイスレポートの実装知見

## pending_writes パターン

Terminal がデバイスレポート（DA1/DA2/DSR/CPR）の応答をPTYに返す必要がある場合、
Terminal 自体は PTY write チャネルを持たず、`pending_writes: Vec<u8>` バッファに蓄積する。
PTY リーダースレッドが `processor.advance()` 後に `drain_pending_writes()` で取り出して送信する。

**利点**: Terminal が I/O に依存しないため、テストが容易。

**セキュリティ**: `MAX_PENDING_WRITES`（4096バイト）の制限を設け、
悪意あるプログラムが大量リクエストでメモリを枯渇させることを防ぐ。
`write_response()` ヘルパーが上限チェックを行う。

## DA1/DA2 intermediate の違い

- DA1 (`CSI c`): intermediate なし。`is_private` (= `?`) ではないことを確認。
- DA2 (`CSI > c`): intermediate に `>` がある。`intermediates.first() == Some(&b'>')` で判定。
  **注意**: `is_private` は `?` のみをチェックするため、DA2 の判定には使えない。

## DECSCUSR の intermediate

`CSI n SP q` のスペース（0x20）が intermediate に入る。
`intermediates.first() == Some(&b' ')` で判定する。

## Alt→ESC prefix

Alt+key は `ESC` + key バイト列として送信する。
Ctrl+key チェックの後に Alt チェックを行い、Ctrl との干渉を避ける。

## ウィンドウタイトル設定時のデッドロック回避

`TerminalState` の Mutex ロックを保持したまま `window.set_title()` を呼ぶとデッドロックする可能性がある。
タイトル文字列をロック内で clone し、ロック解放後に `set_title()` を呼ぶ。
