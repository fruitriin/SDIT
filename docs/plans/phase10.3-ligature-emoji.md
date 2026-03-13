# Phase 10.3: リガチャ + カラー絵文字

**概要**: OpenType リガチャとカラー絵文字の描画を実装する。

**状態: 完了**

## タスク

| タスク | 詳細 | 変更先クレート | 状態 |
|---|---|---|---|
| Phase 10.3a: カラー絵文字 Atlas RGBA 化 | Atlas を Rgba8Unorm に変更、グレースケール昇格、is_color フラグ、シェーダー修正 | sdit-core, sdit | **完了** |
| Phase 10.3b: OpenType リガチャ | 行単位シェーピングでリガチャ検出・複数セル幅グリフ描画 | sdit-core | **完了** |

## 依存関係

なし（独立・任意タイミングで着手可能。Phase 6 以降いつでも）

## Phase 10.3a 実装記録（2026-03-13 完了）

### 変更内容

- `crates/sdit-core/src/render/atlas.rs`: テクスチャフォーマットを `R8Unorm` → `Rgba8Unorm` に変更。`data` を 4 bytes/pixel に。`write()` を RGBA 対応に（行ごとに 4 倍バイト数で書き込み）。`bytes_per_row` を `size * 4` に。テスト `InMemAtlas` も同様に更新、`write_stores_rgba_pixels_correctly` と `write_rejects_wrong_size` テスト追加。
- `crates/sdit-core/src/render/font.rs`: `SwashContent` を import。`GlyphEntry` に `is_color: bool` フィールド追加。`rasterize_glyph()` 内でコンテンツ種別に応じて RGBA 変換（Mask → R=G=B=255,A=alpha / Color → BGRA→RGBA 並び替え / SubpixelMask → RGB→RGBA A=max）。
- `crates/sdit-core/src/render/pipeline.rs`: `CellVertex` に `is_color_glyph: f32` フィールド追加（`@location(7)`）。頂点バッファレイアウトに `offset: 76` のアトリビュート追加。`update_from_grid()` で `entry.is_color` を参照して `is_color_glyph` をセット。
- `crates/sdit-core/src/render/shaders/cell.wgsl`: `CellInput` と `VsOut` に `is_color_glyph` フィールド追加。フラグメントシェーダーを修正（カラーグリフは `mix(bg, texel, texel.a)`、通常グリフは `mix(bg, fg, texel.a)`）。
- `crates/sdit/src/window.rs`, `crates/sdit/src/render.rs`: サイドバー・プリエディット・検索バー描画の `CellVertex` 構築に `is_color_glyph: 0.0` を追加。

### セキュリティ記録

特記事項なし。外部データ（swash ビットマップ）は長さチェック済み。`unsafe` コード不使用。

## Phase 10.3b 実装記録（2026-03-13 完了）

### 設計判断

- 1文字ずつシェーピングする `rasterize_glyph()` ではリガチャを検出できないため、行全体をシェーピングする `shape_line()` メソッドを新設
- cosmic-text の `LayoutGlyph.metadata`（バイトインデックス）と `unicode-width` で cluster 解析し、複数文字→1グリフのリガチャを検出
- ラスタライズロジックを `rasterize_physical_glyph()` ヘルパーに切り出し、`rasterize_glyph()` と `shape_line()` で共有

### 変更内容

- `crates/sdit-core/src/render/font.rs`:
  - `ShapedGlyph` 構造体追加（`start_col`, `num_cells`, `entry`）
  - `shape_line()` メソッド追加 — 行テキスト全体を cosmic-text でシェーピング、cluster 解析でリガチャ検出
  - `rasterize_physical_glyph()` 内部ヘルパー切り出し — SwashContent RGBA 変換 + キャッシュ + Atlas 書き込み
  - `build_byte_to_col_map()` ヘルパー追加 — バイト位置→セルカラムマッピング（CJK 全角対応）
  - テスト追加: `build_byte_to_col_map_ascii`, `build_byte_to_col_map_cjk`
- `crates/sdit-core/src/render/pipeline.rs`:
  - `update_from_grid()` をリガチャ対応に変更 — 行ごとに `shape_line()` → `build_col_glyph_map()` → CellVertex 生成
  - `ColGlyphInfo`, `build_col_glyph_map()`, `glyph_entry_to_vertex_data()` ヘルパー追加
  - リガチャの最初のセル: `cell_width_scale = num_cells`、後続セルは背景のみ
- `crates/sdit-core/src/render/shaders/cell.wgsl`: `cell_width_scale` のクランプ上限を `2.0` → `8.0` に拡張

### セキュリティレビュー結果

| 重要度 | ID | 内容 | 対応 |
|---|---|---|---|
| Medium | M-1 | `build_byte_to_col_map()` での境界外アクセスリスク | **修正済み**: `byte_end = (byte_pos + char_len).min(bytes_len)` で明示的に制限 |
| Medium | M-2 | `shape_line()` での無制限リガチャ幅（DoS リスク） | **修正済み**: `num_cells.min(8)` でシェーダー上限と統一 |
| Medium | M-3 | `update_from_grid()` の整数オーバーフロー | M-2 で自動解決 |
| Low | L-1 | `byte_to_col` アクセスの安全性向上 | 現在も bounds check 済み。将来改善検討 |
| Low | L-2 | シェーダー上限 8.0 との不一致 | M-2 で解決 |
| Low | L-3 | 行末超過リガチャの処理 | 仕様: クリップして描画。`if c < cols` で安全 |
| Info | I-1 | WIDE_CHAR_SPACER 処理変更 | 設計上整合的。shape_line で統一処理 |
