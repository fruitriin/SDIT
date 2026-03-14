# Phase 25.1: ウィンドウリサイズのセル整数倍スナップ

## 要望

ウィンドウをリサイズするとき、セル幅・セル高さの整数倍でのみリサイズされるようにする。
これにより、ウィンドウ端に半端な空白が生まれず、ターミナルグリッドが常に画面にぴったり収まる。

WezTerm: `use_resize_increments = true`（デフォルト: false）
Alacritty: リサイズインクリメントを暗黙的に設定（セルサイズがヒントとなる）

## 現状

SDIT はリサイズインクリメントを設定していないため、ウィンドウを自由にリサイズすると
端数のピクセル領域が余白として残る場合がある。

## 実装方針

1. `WindowAttributes::with_resize_increments()` または `Window::set_resize_increments()` を使う
2. セルサイズ（`cell_width × cell_height`）をインクリメントとして設定
3. ウィンドウ生成時（`window_ops.rs`）とフォントサイズ変更時（`event_loop.rs`）に更新

### 設定

```toml
[window]
resize_increments = true  # デフォルト: false（既存の挙動を維持）
```

macOS では `NSWindow` の `setResizeIncrements:` が呼ばれる（winit が仲介）。

## 変更対象

- `crates/sdit-core/src/config/mod.rs` — `[window] resize_increments: bool` 追加
- `crates/sdit/src/window_ops.rs` — ウィンドウ作成時にリサイズインクリメントを設定
- `crates/sdit/src/event_loop.rs` — フォントサイズ変更時にインクリメントを更新

## 実装結果（2026-03-15 完了）

- `[window] resize_increments = true/false` を config に追加（デフォルト: false）
- ウィンドウ作成時・detach 時・フォントサイズ変更時に `Window::set_resize_increments()` を呼ぶ
- L-1: `cell_width/cell_height` の `is_finite() && > 0` ガードを追加（セキュリティ修正）

テスト: 444 件 PASS

## セキュリティ影響

なし（L-1 修正済み）

## 参照

- WezTerm: `refs/wezterm/config/src/config.rs` `use_resize_increments`
- winit: `Window::set_resize_increments()`
