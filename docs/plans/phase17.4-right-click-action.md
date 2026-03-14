# Phase 17.4: 右クリック動作カスタマイズ

**概要**: 右クリック時の動作を設定で変更可能にする（コンテキストメニュー / ペースト / 無効）。

**状態**: 未着手

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| Config に right_click_action 設定追加 | `context_menu`（デフォルト）/ `paste` / `none` | sdit-core (`config/mod.rs`) | 未着手 |
| 右クリックハンドラー変更 | 設定に応じてコンテキストメニュー表示 or ペースト or 何もしない | sdit (`event_loop.rs`) | 未着手 |
| テスト | 設定デシリアライズ | sdit-core | 未着手 |

## 設定例

```toml
[mouse]
right_click_action = "context_menu"  # context_menu | paste | none
```

## 参照

- `refs/ghostty/src/config/Config.zig` — right-click
