# Phase 6.2: テキスト選択 + クリップボード

**概要**: テキストの選択・コピー・ペースト操作を実装する。

## タスク

| タスク | 詳細 | 変更先クレート |
|---|---|---|
| Selection 型の実装 | `SelectionRange`(start/end Point + block mode)を追加 | sdit-core (新規 `selection.rs`) |
| マウスドラッグでの選択 | 左ボタン押下で選択開始、ドラッグで範囲拡大、ダブルクリックで単語選択、トリプルクリックで行選択 | sdit (`main.rs`) |
| 選択範囲のレンダリング | 選択セルの前景/背景色を反転して描画 | sdit-render (`pipeline.rs`) |
| クリップボード統合 | `arboard` クレート使用。Cmd+C でコピー、Cmd+V でペースト(BRACKETED_PASTE対応) | sdit (`main.rs`) |
| OSC 52 クリップボード操作 | アプリ側からのクリップボード操作を処理 | sdit-core (`terminal/mod.rs`) |

## 依存関係

Phase 6.1（マウスモード判定。ON時はアプリ転送、OFF時に選択動作）

## リファレンス

- `refs/alacritty/alacritty_terminal/src/selection.rs` — Selection 型の設計（最重要）
- `refs/alacritty/alacritty/src/clipboard.rs` — クリップボードプラットフォーム抽象

## 新規依存クレート

`arboard`

## セキュリティ考慮事項

- OSC 52 クリップボード操作は要注意。悪意あるエスケープシーケンスでクリップボードを書き換えるリスクあり → ユーザー確認ダイアログまたは設定で無効化可能にする
