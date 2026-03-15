# Phase 28.3: Alt バッファのホイールスクロール速度設定

## 要望

マウスホイールスクロール時、Alt Screen（full-screen アプリ）では
スクロールイベントをカーソルキーとして PTY に送信する。
この動作の速度（一度に送信するカーソルキー数）を設定できるようにする。

WezTerm: `alternate_buffer_wheel_scroll_speed = 3`（デフォルト: 3）

## 現状

SDIT では Alt Screen でのホイールスクロールをカーソルキーに変換して送信しているが、
速度は固定値。ユーザーが調整できない。

## 実装方針

1. `[scrolling]` または `[mouse]` セクションに `alt_scroll_speed: u8 = 3` 追加
2. Alt Screen 時のホイールスクロール処理で `alt_scroll_speed` 回カーソルキーを送信

## 変更対象

- `crates/sdit-core/src/config/mod.rs` — `alt_scroll_speed` 追加
- `crates/sdit/src/event_loop.rs` — Alt Screen スクロール時に設定値を参照

## セキュリティ影響

なし（上限 255 に制限）

## 参照

- WezTerm: `refs/wezterm/config/src/config.rs` `alternate_buffer_wheel_scroll_speed`
