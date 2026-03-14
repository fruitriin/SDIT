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

## 実装結果（2026-03-15 完了）

### 実装した変更

`crates/sdit/src/window.rs` の PTY リーダースレッドを修正:
- `drain_pending_writes()` の結果を Mutex ロック内で変数に回収
- ロック解放後（`}` の外）で `write_tx.send(response)` を呼ぶよう変更
- Mutex 保持中の SyncSender.send() ブロッキングを根本解消

### テスト結果
- 基本 echo 動作、DA クエリ後の応答性、マルチウィンドウ操作すべて PASS ✓
- リグレッション: 001-basic-echo、003-multi-window PASS ✓
- セキュリティレビュー: Critical/High 0件、M-3 は影響限定的として Plan に記録 ✓

## セキュリティ影響

なし（パフォーマンス・安定性の改善）

---

## セキュリティレビュー結果（2026-03-15）

### 総合判定: ✅ セキュリティ脆弱性なし（Medium改善案あり）

| ID | カテゴリ | 重要度 | 課題 | 対応 |
|---|---|---|---|---|
| M-3 | メモリ安全 | Medium | pending_writes の容量制限なし | 独立計画に分離（影響限定的） |
| L-1 | 例外処理 | Low | Mutex poison 時の継続 | 設計上許容 |
| L-2 | イベント信頼性 | Low | event_proxy 送出エラー無視 | 稀なケース |

### 主要発見

**M-3 (Medium): pending_writes の容量制限**
- `drain_pending_writes()` が回収する応答バイト列に上限がない
- VTE パーサーが短時間に複数クエリを受けた場合（DA, DSR, CPR 連続）、応答が蓄積してメモリ圧力増大の可能性
- 影響度は低いが、改善余地あり

**M-3 の判断**: `drain_pending_writes()` は毎 PTY 読み取りイテレーション（8192 バイト）ごとに呼ばれ蓄積しない。
DA/DSR 等の応答は数十バイト程度で実際のメモリ圧力は限定的。このフェーズではブロッキングなし。
将来対応として `docs/plans/phase21.4b-pty-pending-writes-bound.md` を必要に応じて作成する。

### 修正検証済み項目

✅ Mutex スコープが明確に制限（RAII）
✅ データレース無し（値セマンティクス）
✅ デッドロック主原因の完全除去
✅ バッファオーバーフロー無し
✅ パニック時の poison 処理適切
