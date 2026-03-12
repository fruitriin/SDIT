# Phase 7: IME入力サポート

**概要**: macOS IME(日本語入力)に対応する。CJKフォント対応は済んでいるため、入力側の対応が急務。

## タスク

| タスク | 詳細 | 変更先クレート |
|---|---|---|
| winit IME イベント処理 | `WindowEvent::Ime(Preedit, Commit)` を処理し、Commit 時にPTYへ書き込み | sdit (`main.rs`) |
| IME 有効化 | `Window::set_ime_allowed(true)` + `set_ime_cursor_area()` でカーソル位置通知 | sdit (`main.rs`) |
| プリエディット表示 | 変換候補をカーソル位置にインライン描画 | sdit-render |

## 依存関係

なし（Phase 6と並行可能だが、Phase 6後を推奨）

## リファレンス

- `refs/alacritty/alacritty/src/input/mod.rs` — IME イベントハンドリング
- winit 公式ドキュメント(`WindowEvent::Ime`)
