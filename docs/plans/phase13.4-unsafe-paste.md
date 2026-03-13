# Phase 13.4: Unsafe Paste 警告

**概要**: 改行を含むペースト時に確認ダイアログを表示するセキュリティ機能。

**状態**: **完了**

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| PasteConfig 設定項目 | `paste.confirm_multiline` (default: true) | sdit-core (`config/mod.rs`) | 完了 |
| is_unsafe_paste 判定 | 改行 + bracketed paste 脱出シーケンス検出 | sdit (`app.rs`) | 完了 |
| confirm_unsafe_paste ダイアログ | rfd で警告ダイアログ表示 + プレビュー | sdit (`app.rs`) | 完了 |
| Action::Paste 統合 | ペースト前に unsafe check + 確認 | sdit (`event_loop.rs`) | 完了 |
| ユニットテスト | PasteConfig serde 2件 + is_unsafe_paste 3件 | sdit-core, sdit | 完了 |

## 依存クレート追加

- `rfd = "0.15"` — ネイティブダイアログ（macOS NSAlert / GTK / Win32）

## 参照

- `refs/ghostty/src/input/paste.zig`

## 依存関係

なし

## セキュリティレビュー結果

### M-1: bracketed paste 脱出インジェクション検出不足（Medium）— 修正済み

`\x1b[201~`（bracketed paste 終了シーケンス）は bracketed paste モード有効時でも脱出に使える。

**修正**: `is_unsafe_paste` で `\x1b[201~` を bracketed_paste_mode チェックより先に検出。

### L-1: ダイアログプレビューの制御文字表示（Low）— 修正済み

**修正**: プレビュー生成時に制御文字を `·` に置換。

### L-2: rfd ダイアログの同期ブロッキング（Low）

macOS NSAlert の標準的な動作。将来的に `AsyncMessageDialog` への移行を検討。

### L-3: Plan の設定キー名と実装の不一致（Low）— 修正済み

Plan を `confirm_multiline` に更新。

### I-1: IME コミット経由のペーストへの非適用（Info）

設計上許容。IME コミットは通常1文字〜数文字。

### I-2: rfd Custom バリアント比較（Info）

フェイルセーフ（拒否側）の動作は正しい。
