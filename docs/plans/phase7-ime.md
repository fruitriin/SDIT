# Phase 7: IME入力サポート

**概要**: macOS IME(日本語入力)に対応する。CJKフォント対応は済んでいるため、入力側の対応が急務。

**状態**: **完了**（2026-03-12）

## タスク

| タスク | 詳細 | 変更先クレート | 状態 |
|---|---|---|---|
| winit IME イベント処理 | `WindowEvent::Ime(Preedit, Commit)` を処理し、Commit 時にPTYへ書き込み | sdit (`event_loop.rs`) | **完了** |
| IME 有効化 | `Window::set_ime_allowed(true)` + `set_ime_cursor_area()` でカーソル位置通知 | sdit (`window_ops.rs`, `render.rs`) | **完了** |
| プリエディット表示 | 変換候補をカーソル位置にインライン描画 | sdit (`render.rs`) | **完了** |

## 実装詳細

### 変更ファイル

- `crates/sdit/src/app.rs`: `PreeditState` 構造体と `SditApp.preedit` フィールドを追加
- `crates/sdit/src/window_ops.rs`: `create_window()` と `detach_session_to_new_window()` で `set_ime_allowed(true)` を呼ぶ
- `crates/sdit/src/event_loop.rs`: `WindowEvent::Ime(Commit|Preedit|Enabled|Disabled)` を処理
- `crates/sdit/src/render.rs`: IME カーソル位置通知 + プリエディット描画ロジック + `char_cell_width()` ヘルパー
- `crates/sdit-core/src/render/pipeline.rs`: `CellPipeline::overwrite_cell()` メソッドを追加

### セキュリティ考慮事項

- `Ime::Commit` 時、BRACKETED_PASTE モードが有効かつテキスト長が1より大きい場合はブラケットシーケンスをサニタイズしてから送信（Terminal Injection 攻撃防止）
- **M-1 修正済み**: `wrap_bracketed_paste()` をループ方式に変更（ダブルリプレースバイパス対策）。ペースト処理と共通ヘルパーに統合
- **L-1 修正済み**: `ime_commit_to_bytes()` で `text.len()` → `text.chars().count()` に修正（バイト数ではなく文字数で判定）
- **I-1 修正済み**: `render.rs` の不要な `preedit.clone()` を借用に変更

### 制限事項と Low/Info 項目

- **L-2**: `char_cell_width()` は主要な CJK 範囲のみをカバー。`unicode-width` クレートを使えば完全対応できる（将来の改善として残す）
- **L-3**: プリエディット中のカーソル位置ハイライト（IME カーソル下線）は未実装。winit の `cursor_offset` を使ったインライン下線描画は Phase 8 以降で対応
- **L-4**: プリエディットテキストの長さ上限なし（悪意ある IME からの DoS 可能性は極めて低い）
- **L-5**: `char_cell_width()` の絵文字範囲 `0x1FA00..=0x1FAFF` が未カバー
- **Info**: `PreeditState.cursor_offset` フィールドは現時点で未使用（将来の下線描画用）

## 依存関係

なし（Phase 6と並行可能だが、Phase 6後を推奨）

## リファレンス

- `refs/alacritty/alacritty/src/input/mod.rs` — IME イベントハンドリング
- winit 公式ドキュメント(`WindowEvent::Ime`)
