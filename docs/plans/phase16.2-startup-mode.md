# Phase 16.2: 起動モード設定

**概要**: 起動時にウィンドウを最大化またはフルスクリーンで開始する設定を追加する。

**状態**: **完了**

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| WindowConfig に startup_mode 追加 | `Windowed`/`Maximized`/`Fullscreen` enum | sdit-core (`config/mod.rs`) | **完了** |
| ウィンドウ生成時に適用 | winit `set_maximized(true)` / `set_fullscreen()` | sdit (`window_ops.rs`) | **完了** |
| テスト | 設定デシリアライズ + デフォルト値 | sdit-core | **完了** |

## 設定例

```toml
[window]
startup_mode = "Windowed"  # Windowed | Maximized | Fullscreen
```

## 参照

- `refs/alacritty/alacritty/src/config/window.rs` — startup_mode
- `refs/ghostty/src/config/Config.zig` — maximize, fullscreen
