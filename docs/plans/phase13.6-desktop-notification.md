# Phase 13.6: デスクトップ通知

**概要**: OSC 9 / OSC 99 でシステム通知を発行。長時間コマンド完了通知に有用。

**状態**: **完了**

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| OSC 9 パース | iTerm2 互換（本文のみ） | sdit-core (`terminal/mod.rs`) | 完了 |
| OSC 99 パース | Kitty 互換（簡易実装: 本文のみ） | sdit-core (`terminal/mod.rs`) | 完了 |
| SditEvent | DesktopNotification { title, body } | sdit (`app.rs`) | 完了 |
| PTY リーダー統合 | take_notification → イベント送出 | sdit (`window.rs`) | 完了 |
| notify-rust 通知 | 別スレッドで非同期送信 + AtomicBool レート制限 | sdit (`event_loop.rs`) | 完了 |
| NotificationConfig | `notification.enabled` (default: true) | sdit-core (`config/mod.rs`) | 完了 |
| テスト | OSC パース 7件 + Config serde 2件 | sdit-core | 完了 |

## 依存クレート追加

- `notify-rust = "4"` — クロスプラットフォームデスクトップ通知

## 依存関係

なし

## セキュリティレビュー結果

### M-1: 通知テキストの制御文字サニタイズ（Medium）— 修正済み

**修正**: `sanitize_notification_text()` で制御文字を除去（改行・タブは保持）。

### M-2: 通知スレッドのリーク（Medium）— 修正済み

**修正**: `AtomicBool` による in-flight フラグで同時実行を1スレッドに制限。

### L-1: UTF-8 バイト境界でのトランケーション（Low）— 修正済み

**修正**: `truncate_utf8()` で `valid_up_to()` を使い安全なバイト境界まで戻す。

### L-2: OSC 99 がコメントと乖離（Low）

簡易実装として本文のみを処理。Kitty の key=value 形式は未パース。

### L-3: スレッド名固定（Low）

M-2 の修正で同時スレッド数が1に制限されるため実質解消。

### I-1: デフォルト有効による情報露出（Info）

共有端末環境では `enabled = false` を推奨。
