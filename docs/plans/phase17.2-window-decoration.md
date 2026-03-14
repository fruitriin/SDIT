# Phase 17.2: ウィンドウデコレーション設定

**概要**: タイトルバー・ウィンドウフレームの表示/非表示を設定・トグルできるようにする。

**状態**: 未着手

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| Config に decorations 設定追加 | `full`（デフォルト）/ `none` / `transparent`（macOS） | sdit-core (`config/mod.rs`) | 未着手 |
| winit 連携 | WindowAttributes::with_decorations() で初期設定適用 | sdit (`window_ops.rs`) | 未着手 |
| Action::ToggleDecorations | キーバインドで動的に切り替え | sdit-core (`config/keybinds.rs`), sdit (`event_loop.rs`) | 未着手 |
| テスト | 設定デシリアライズ | sdit-core | 未着手 |

## 設定例

```toml
[window]
decorations = "full"  # full | none | transparent (macOS)
```

## 参照

- `refs/ghostty/src/apprt/action.zig` — toggle_window_decorations
- `refs/alacritty/alacritty/src/config/window.rs` — Decorations enum
