# Phase 28.2: テキスト点滅設定（SGR BLINK 属性）

## 要望

SGR 5（BLINK）/ SGR 6（RAPID_BLINK）属性を持つテキストを実際に点滅させる。
現在 SDIT は BLINK フラグを保持しているが、視覚的に点滅しない。

WezTerm: `text_blink_rate`, `text_blink_rapid_rate`（ms）、`text_blink_ease_in/out`

## 現状

`CellFlags::BLINK` は存在するが、レンダラーが点滅アニメーションを実装していない。
ほとんどのモダンターミナルはデフォルトで点滅を無効にしており、SDIT でも
オプションで有効化できる形が望ましい。

## 実装方針

1. `[colors]` または `[terminal]` 設定に `text_blink = false` を追加（デフォルト: false、無効）
2. `text_blink_rate = 500` — 点滅周期（ミリ秒）
3. レンダラーで `CellFlags::BLINK` が立っているセルを、時刻に基づいて表示/非表示を切り替える

### 実装注意

- 点滅が有効なウィンドウは定期的な再描画が必要（CPU 使用量増加）
- cursor blink の既存実装（`cursor_blink_visible`, `cursor_blink_last_toggle`）を参考にする

## 変更対象

- `crates/sdit-core/src/config/mod.rs` — `text_blink: bool`, `text_blink_rate: u32` 追加
- `crates/sdit/src/app.rs` — `text_blink_visible: bool`, `text_blink_last_toggle` フィールド追加
- `crates/sdit/src/render.rs` — 点滅セルの表示制御
- `crates/sdit/src/event_loop.rs` — 定期再描画（`NewEvents` で点滅タイマー確認）

## セキュリティ影響

なし

## 参照

- WezTerm: `text_blink_rate`, `text_blink_ease_in`
- SGR 5（BLINK）/ SGR 6（RAPID_BLINK）
