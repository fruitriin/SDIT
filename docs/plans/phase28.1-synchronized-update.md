# Phase 28.1: CSI ?2026 Synchronized Update Mode（同期レンダリング）

## 要望

プライベートモード 2026（Synchronized Output）に対応する。
ターミナルアプリがこのモードを有効化すると、出力をバッファリングして
一括描画することでフリッカーを防ぐ。

Alacritty、Ghostty、WezTerm、Kitty が実装済み。
`less`, `vim`, `htop` 等の CUI ツールが対応を開始している。

## 現状

SDIT は CSI ?2026h / ?2026l シーケンスを無視している。
ターミナルアプリが同期モードを要求しても、即座に描画してしまいフリッカーが発生する可能性がある。

## 実装方針

1. `TermMode` に `SYNCHRONIZED_OUTPUT` フラグを追加
2. CSI ?2026h → フラグを立てる（「バッファリングモード開始」）
3. CSI ?2026l → フラグを下ろして描画をトリガー
4. `PtyOutput` イベント処理時: `SYNCHRONIZED_OUTPUT` が立っていれば `redraw_session` をスキップ
5. モード解除時（?2026l）に強制描画

### DA1 レスポンスの更新

CSI ?2026 をサポートする場合、DA1（Device Attributes）レスポンスに
`\x1b[?2026;...c` 等の形式でサポートを通知する必要がある。

## 変更対象

- `crates/sdit-core/src/terminal/` — `TermMode::SYNCHRONIZED_OUTPUT` フラグ追加、?2026h/?2026l 処理
- `crates/sdit/src/event_loop.rs` — `PtyOutput` イベント処理で sync フラグを確認

## セキュリティ影響

なし

## 参照

- Alacritty: `refs/alacritty/alacritty-terminal/src/term/mod.rs` — TermMode フラグ
- Ghostty: `refs/ghostty/src/terminal/modes.zig` — SYNCHRONIZED_OUTPUT
- WezTerm: synchronized output support
- https://gitlab.freedesktop.org/terminal-wg/specifications/-/merge_requests/2
