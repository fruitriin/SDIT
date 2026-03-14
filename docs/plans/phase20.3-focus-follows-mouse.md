# Phase 20.3: Focus Follows Mouse

**概要**: マウスがウィンドウに乗り入れたとき自動的にフォーカスする機能を追加する。

**状態**: 未着手

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| Config に focus_follows_mouse 追加 | bool (デフォルト false) | sdit-core (`config/mod.rs`) | 未着手 |
| CursorEntered イベントで focus() 呼び出し | winit の WindowEvent::CursorEntered | sdit (`event_loop.rs`) | 未着手 |
| テスト | 設定デシリアライズ | sdit-core | 未着手 |

## 設定例

```toml
[mouse]
focus_follows_mouse = false
```

## 参照

- `refs/ghostty/src/config/Config.zig` — focus-follows-mouse
