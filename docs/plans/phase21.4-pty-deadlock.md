# Phase 21.4: PTY リーダーのデッドロック的停滞を修正

## 問題

SDIT 上で Claude Code を起動してプロンプトを投げると数秒でフリーズする（優先度低）。

## 原因

PTY リーダースレッドが `Mutex<TerminalState>` をロックした状態で `write_tx.send(response)` を呼ぶ。`write_tx` は容量 64 の `SyncSender` で、チャンネルが満杯になるとブロッキングする。

Claude Code / ink は接続時に DA, DA2, XTVERSION, Kitty keyboard query 等の問い合わせを連続発行する。SDIT が `pending_writes` で応答しようとした際に、PTY ライタースレッドの消費が追いつかずチャンネルが埋まり、リーダースレッドが Mutex 保持のままブロック → メインスレッドの `redraw_session()` が同 Mutex を取得できず停滞する。

### 副因

- 8192 バイト読み込みごとに `PtyOutput` イベントを送出し、高頻度描画ストームが発生する
- VTE パーサーが 1 バイトずつ処理（`advance` のループ）で Mutex ロック保持時間が長い

## 修正方針

1. **Mutex ロック外で write_tx.send**: `pending_writes` を Mutex ロック内で回収し、ロック解放後にまとめて `write_tx.send` する
2. **`try_send` への変更を検討**: ブロッキングを回避し、溢れた応答は次回ループで再試行
3. **PtyOutput イベントのバッチ化を検討**: 短時間の連続出力を1イベントに集約（描画頻度の抑制）

## 変更対象

- `crates/sdit/src/window.rs` — PTY リーダースレッドの `pending_writes` 処理
- `crates/sdit/src/app.rs` — `SyncSender` の容量設定（必要に応じて）

## セキュリティ影響

なし（パフォーマンス・安定性の改善）
