# Phase 20.3: Focus Follows Mouse

**概要**: マウスがウィンドウに乗り入れたとき自動的にフォーカスする機能を追加する。

**状態**: 完了

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| Config に focus_follows_mouse 追加 | bool (デフォルト false) | sdit-core (`config/mod.rs`) | 完了 |
| CursorEntered イベントで focus() | has_focus() チェック付き | sdit (`event_loop.rs`) | 完了 |
| テスト | 設定デシリアライズ | sdit-core | 完了 |

## 設定例

```toml
[mouse]
focus_follows_mouse = false
```

## セキュリティレビュー結果

| 重要度 | ID | 内容 | 対応 |
|---|---|---|---|
| Medium | M-2 | 二重フォーカス防止 | 修正済み: has_focus() チェックを追加 |

## 参照

- `refs/ghostty/src/config/Config.zig` — focus-follows-mouse
