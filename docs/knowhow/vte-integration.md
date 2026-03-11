# VTE パーサー統合の知見

## vte 0.13 の Perform trait

vte 0.13 は低レベルの `Perform` trait を提供する:
- `print(c)` — 表示文字
- `execute(byte)` — C0 制御文字 (BS, HT, LF, CR 等)
- `csi_dispatch(params, intermediates, ignore, action)` — CSI シーケンス
- `esc_dispatch(intermediates, ignore, byte)` — ESC シーケンス
- `osc_dispatch(params, bell_terminated)` — OSC シーケンス

Alacritty は独自の `Handler` trait で高レベル化しているが、SDIT では Terminal に直接 Perform を実装した。

## 実装上の注意点

### `input_needs_wrap` フラグ
- `print()` で右端到達時に true にセット
- 次の `print()` で改行を実行してからセルに書き込む
- カーソル移動系の操作では false にリセット

### SGR パラメータ解析
- `38;5;N` (256色) と `38;2;R;G;B` (RGB) は2つの形式がある:
  1. セミコロン区切り: `38;5;N` → 別々のパラメータとして到達
  2. コロン区切り: `38:5:N` → 1つのパラメータのサブパラメータとして到達
- 両方をサポートする必要がある

### セキュリティ上の教訓
- `Line(i32)` → `usize` 変換は `Line::as_viewport_idx()` を使う（`.0.max(0) as usize` の統一化）
- `scroll_up/down` は `region.end <= region.start` を早期リターン
- `Grid::new` で `lines = 0` は `1` にクランプ（ゼロ除算防止）
- `CUU/CUD` のカーソル移動は `saturating_sub/add` を使う

### OSC パーサーの注意点
- vte 0.13 は OSC パラメータのバッファを内部で ~1024 バイトに制限している
- SDIT 側でも `MAX_TITLE_BYTES = 4096` の防御を入れている（vte 変更時の保険）
- テストでは `Perform::osc_dispatch` を直接呼んで上限を検証する（vte 経由だとパーサーのバッファ制限が先に効く）

## テスト構成

| テスト種別 | ファイル | テスト数 |
|---|---|---|
| ユニット（grid系） | `grid/{cell,row,storage,mod}.rs` | 32 |
| ユニット（index） | `index.rs` | 6 |
| ユニット（pty） | `pty/mod.rs` | 7 |
| ユニット（terminal） | `terminal/mod.rs` | 15 |
| 統合テスト | `tests/headless_pipeline.rs` | 3 |
| 合計 | — | 63（全通過） |
