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

## 実装完了

2026-03-12 実装完了。

### 実装内容

- `crates/sdit-core/src/selection.rs`: `Selection` 型 (Simple/Word/Lines)、`selected_text()` 関数、`contains()`/`to_tuple()` メソッド
- `crates/sdit-core/src/terminal/mod.rs`: OSC 52 クリップボード書き込み処理、`take_clipboard_write()`、`decode_base64()` ヘルパー
- `crates/sdit/src/app.rs`: `selection: Option<Selection>`、クリップボードフィールド、クリック追跡フィールド
- `crates/sdit/src/event_loop.rs`: Cmd+C（コピー）、Cmd+V（ペースト）、ダブル/トリプルクリック選択、`expand_word()` ヘルパー
- `crates/sdit/src/input.rs`: `is_copy_shortcut()`、`is_paste_shortcut()` 追加
- `crates/sdit/src/window.rs`: OSC 52 イベント発行
- `crates/sdit/Cargo.toml`: `arboard = "3"` 追加

### セキュリティ考慮事項

- OSC 52 書き込みのみ許可（読み取り `?` は無応答）
- クリップボード上限 1 MiB で制限
- **Info I-1**: OSC 52 書き込みを config で無効化する設定を将来追加すること
- **Info I-2**: `decode_base64` は padding なし Base64 を受け付けない場合がある（標準的な実装として許容）
