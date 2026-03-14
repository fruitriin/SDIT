# Phase 17.3: Always On Top

**概要**: ウィンドウを常に最前面に固定する機能を追加する。

**状態**: 未着手

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| Config に always_on_top 設定追加 | `bool`（デフォルト false） | sdit-core (`config/mod.rs`) | 未着手 |
| winit 連携 | Window::set_window_level() で FloatingWindow レベル設定 | sdit (`window_ops.rs`, `event_loop.rs`) | 未着手 |
| Action::ToggleAlwaysOnTop | キーバインドで動的にトグル | sdit-core (`config/keybinds.rs`), sdit (`event_loop.rs`) | 未着手 |
| テスト | 設定デシリアライズ | sdit-core | 未着手 |

## 設定例

```toml
[window]
always_on_top = false
```

## 参照

- `refs/ghostty/src/apprt/action.zig` — float_window
