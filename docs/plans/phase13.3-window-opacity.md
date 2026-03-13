# Phase 13.3: 背景透過 + macOS blur

**概要**: ウィンドウ背景の不透明度設定。macOS ユーザーに人気の高い視覚カスタマイズ。

**状態**: **完了**

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| WindowConfig 構造体 | `opacity: f32`, `blur: bool`, `clamped_opacity()` | sdit-core (`config/mod.rs`) | 完了 |
| Config に window フィールド | `pub window: WindowConfig` | sdit-core (`config/mod.rs`) | 完了 |
| save_with_comments | `[window]` セクションコメント | sdit-core (`config/mod.rs`) | 完了 |
| ウィンドウ透過設定 | `with_transparent()` + `with_blur()` | sdit (`window_ops.rs`) | 完了 |
| clear_color alpha | opacity を alpha チャンネルに反映 | sdit (`event_loop.rs`) | 完了 |
| Hot Reload | opacity/blur 変更時にウィンドウ更新 | sdit (`app.rs`) | 完了 |
| ユニットテスト | default/deserialize/partial/clamp/nan_inf 5件 | sdit-core | 完了 |

## 参照

- `refs/alacritty/alacritty/src/config/window.rs`

## 依存関係

なし

## セキュリティレビュー結果

### M-1: opacity に NaN/Inf を渡した場合のクランプ不備（Medium）— 修正済み

`f32::NAN.clamp(0.0, 1.0)` は NaN を返し、GPU ドライバで未定義動作を引き起こす。

**修正**: `clamped_opacity()` に `is_finite()` チェックを追加。非有限値は 1.0 を返す。

### M-2: BEL ボムによるイベントキュー枯渇（Medium）— 修正済み

PTY リーダーが BEL を無制限にイベントキューに送出でき、大量の BEL でイベントループが遅延する。

**修正**: PTY リーダースレッドに 100ms のレート制限を追加。

### L-1: clear_color RGB チャンネルの範囲外リスク（Low）

HDR 等で背景色が 1.0 を超えた場合にクリアカラーが範囲外になる。現時点では実害低い。

### L-2: opacity=1.0 でも透過フラグが常時有効（Low）— 修正済み

**修正**: `needs_transparent = clamped_opacity() < 1.0 || blur` の条件で設定。

### L-3: Hot Reload の opacity 比較に EPSILON 使用（Low）

f32::EPSILON が小さすぎて丸め誤差で不要な再描画が発生する可能性。実害は軽微。

### I-1: VisualBell::completed() がデッドコード（Info）

現在未使用。将来のクリーンアップ時に対処。
