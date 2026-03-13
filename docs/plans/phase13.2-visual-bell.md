# Phase 13.2: ビジュアルベル + システム通知

**概要**: BEL (0x07) 受信時の視覚フィードバック。現在はログ出力のみ。

**状態**: **完了**

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| BellConfig 設定項目 | `bell.visual`, `bell.dock_bounce`, `bell.duration_ms` | sdit-core (`config/mod.rs`) | 完了 |
| SditEvent::BellRing | BEL 検出時のイベント伝播 | sdit (`app.rs`, `window.rs`) | 完了 |
| VisualBell 構造体 | ring/intensity/completed メソッド、レート制限 | sdit (`app.rs`) | 完了 |
| ビジュアルベル描画 | clear_color と白を intensity で補間、フェードアニメーション | sdit (`event_loop.rs`) | 完了 |
| macOS Dock バウンス | `Window::request_user_attention(Informational)` | sdit (`event_loop.rs`) | 完了 |
| Hot Reload 対応 | bell 設定変更時に VisualBell の duration を更新 | sdit (`app.rs`) | 完了 |
| ユニットテスト | BellConfig serde 3件 + VisualBell 5件 + clamp 2件 | sdit-core, sdit | 完了 |

## 参照

- `refs/alacritty/alacritty/src/display/bell.rs`

## 依存関係

なし

## セキュリティレビュー結果

### M-1: BEL bomb レート制限（Medium）— 修正済み

悪意あるプロセスが高頻度で BEL を送信すると、`ring()` が毎回 `start_time` をリセットし、
アニメーションが永続的にリスタートして CPU 使用率が上昇する。

**修正**: `ring()` でアニメーション進行中（`intensity_inner() > 0.0`）なら新しいリングを無視。

### L-1: duration_ms = 0 による NaN（Low）— 修正済み

`duration_ms = 0` の場合、`intensity_inner()` で 0.0/0.0 = NaN が発生する。

**修正**: `intensity_inner()` に `self.duration.is_zero()` ガードを追加。

### L-2: duration_ms 上限なし（Low）— 修正済み

巨大な `duration_ms` で長時間の継続再描画ループが発生する。

**修正**: `BellConfig::clamped_duration_ms()` で 1..5000 にクランプ。全呼び出し箇所で使用。

### I-1: bell_pending フラグのスレッド安全性（Info）

`bell_pending` は `Mutex<TerminalState>` 内にあり、PTY リーダースレッドからのアクセスは
Mutex で保護されている。現状問題なし。

### I-2: Dock バウンスのフォーカス判定（Info）

`has_focus()` でフォーカス判定しているが、フォーカス遷移中のタイミング次第で
バウンスが発生しないケースがありうる。実害は軽微。
