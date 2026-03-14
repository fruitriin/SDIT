# Phase 16.3: 閉じる前の確認ダイアログ

**概要**: セッションを閉じるとき（プロセスが実行中の場合）に確認ダイアログを表示する。

**状態**: 未着手

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| Config に confirm_close 設定追加 | `never`/`always`/`process_running`（デフォルト） | sdit-core (`config/mod.rs`) | 未着手 |
| プロセス実行中判定 | シェルプロセスの子プロセスの有無を確認 | sdit-core (`session/`) | 未着手 |
| 確認ダイアログ表示 | macOS: NSAlert、他: ターミナル内オーバーレイ | sdit (`event_loop.rs`) | 未着手 |
| CloseSession/Quit アクションの変更 | 確認が必要な場合はダイアログ表示→確認後に実行 | sdit (`event_loop.rs`) | 未着手 |
| テスト | 設定デシリアライズ | sdit-core | 未着手 |

## 設定例

```toml
[window]
confirm_close = "process_running"  # never | always | process_running
```

## 参照

- `refs/ghostty/src/config/Config.zig` — confirm-close-surface
- `refs/wezterm/wezterm-gui/src/overlay/confirm_close_pane.rs`
