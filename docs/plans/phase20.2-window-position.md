# Phase 20.2: ウィンドウ座標保存

**概要**: ウィンドウの位置を保存・復帰する機能を追加する。

**状態**: 未着手

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| Config に window_position 追加 | Option<(i32, i32)> | sdit-core (`config/mod.rs`) | 未着手 |
| ウィンドウ生成時に位置を設定 | winit の with_position() を使用 | sdit (`event_loop.rs`) | 未着手 |
| ウィンドウ移動時に位置を保存 | window-persistence と同様のパターン | sdit (`event_loop.rs`) | 未着手 |
| テスト | 設定デシリアライズ | sdit-core | 未着手 |

## 設定例

```toml
[window]
position_x = 100
position_y = 200
```

## 参照

- `refs/ghostty/src/config/Config.zig` — window-position-x, window-position-y
- Phase 10.2 (window-persistence) の実装パターン
