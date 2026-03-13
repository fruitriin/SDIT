# Phase 13.1: macOS Option as Alt

**概要**: macOS の Option キーを Alt として扱う設定。readline ショートカット（Alt+B/F/D 等）に必須。

**状態**: **完了**

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| 設定項目追加 | `option_as_alt: Both/Left/Right/None` | sdit-core (`config/mod.rs`) | 完了 |
| winit 連携 | `Window::set_option_as_alt()` でフラグ反映 | sdit (`window_ops.rs`) | 完了 |
| Hot Reload 対応 | 設定変更時に全ウィンドウへリアルタイム反映 | sdit (`app.rs`) | 完了 |

## 参照

- `refs/alacritty/alacritty/src/config/window.rs`
- `refs/ghostty/src/input/keyboard.zig`

## 依存関係

なし
