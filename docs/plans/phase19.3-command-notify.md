# Phase 19.3: コマンド終了通知

**概要**: シェルインテグレーション（OSC 133）と連携し、長時間実行コマンドの終了時にデスクトップ通知を送る機能を追加する。

**状態**: 未着手

## 前提条件

- Phase 14.5（シェルインテグレーション OSC 133）が実装済みであること

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| Config に command_notify 設定追加 | threshold_seconds (デフォルト 10), enabled | sdit-core (`config/mod.rs`) | 未着手 |
| コマンド開始/終了の追跡 | OSC 133 の prompt mark でコマンド実行時間を計測 | sdit-core (`terminal/mod.rs`) | 未着手 |
| 通知送信 | notify-rust で通知、コマンド名・実行時間・終了コードを含む | sdit (`event_loop.rs`) | 未着手 |
| テスト | 設定デシリアライズ + 時間計測ロジック | sdit-core | 未着手 |

## 設定例

```toml
[notification]
command_notify = true
command_notify_threshold = 10  # 秒
```

## 参照

- `refs/ghostty/src/config/Config.zig` — notify-on-command-finish
