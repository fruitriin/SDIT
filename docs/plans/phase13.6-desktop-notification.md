# Phase 13.6: デスクトップ通知

**概要**: OSC 9 / OSC 99 でシステム通知を発行。長時間コマンド完了通知に有用。

**状態**: 未着手

## タスク

| タスク | 詳細 | 変更先 |
|---|---|---|
| OSC 9/99 パース | 通知タイトル・本文の抽出 | sdit-core (`terminal/`) |
| macOS 通知連携 | `UNUserNotificationCenter` API | sdit (`event_loop.rs`) |
| 設定項目 | `notification.enabled = true/false` | sdit-core (`config/`) |

## 依存関係

なし
