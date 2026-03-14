# Phase 19.3: コマンド終了通知

**概要**: シェルインテグレーション（OSC 133）と連携し、長時間実行コマンドの終了時にデスクトップ通知を送る機能を追加する。

**状態**: 完了

## 前提条件

- Phase 14.5（シェルインテグレーション OSC 133）が実装済みであること

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| Config に command_notify 設定追加 | CommandNotifyMode (never/unfocused/always), threshold_seconds (デフォルト 10) | sdit-core (`config/mod.rs`) | 完了 |
| コマンド開始/終了の追跡 | OSC 133;B で Instant 記録、OSC 133;D で経過時間を計算 | sdit-core (`terminal/mod.rs`) | 完了 |
| 通知送信 | CommandFinished イベント → 既存 DesktopNotification パスで notify-rust 通知 | sdit (`event_loop.rs`) | 完了 |
| テスト | 設定デシリアライズ + 時間計測ロジック（5件追加） | sdit-core | 完了 |

## 設定例

```toml
[notification]
command_notify = "unfocused"  # "never" | "unfocused" | "always"
command_notify_threshold = 10  # 秒（1〜3600）
```

## 実装メモ

- `CommandNotifyMode` enum: `Never` / `Unfocused` / `Always`（kebab-case serde）
- Terminal に `command_start_time: Option<Instant>` と `command_finished_pending: Option<(u64, Option<i32>)>` を追加
- 閾値判定は GUI 側（event_loop.rs）で実施（Terminal は Config への参照を持たないため）
- `Unfocused` モード: session_id に対応するウィンドウがフォーカスされていない場合のみ通知
- 既存の `DesktopNotification` パス（AtomicBool レート制限含む）を再利用

## セキュリティレビュー結果

| 重要度 | ID | 内容 | 対応 |
|---|---|---|---|
| Low | L-1 | 閾値を 1 秒に設定可能（通知フラッド） | 記録のみ: デフォルト 10 秒、AtomicBool レート制限あり |
| Low | L-2 | send_event() 失敗時のログ出力なし | 記録のみ: 既存パターンと同一 |
| Low | L-3 | command_start_time の長時間保持 | 記録のみ: メモリリークではなく Instant 1個分 |
| Info | I-1 | セッション存在確認の race condition | 記録のみ: Rust 所有権により安全 |

## 参照

- `refs/ghostty/src/config/Config.zig` — notify-on-command-finish
