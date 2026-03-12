# Phase 10.3: リガチャ + カラー絵文字

**概要**: OpenType リガチャとカラー絵文字の描画を実装する。

## タスク

| タスク | 詳細 | 変更先クレート | 状態 |
|---|---|---|---|
| Phase 10.3a: カラー絵文字 Atlas RGBA 化 | Atlas を Rgba8Unorm に変更、グレースケール昇格、is_color フラグ、シェーダー修正 | sdit-core, sdit | **完了** |
| OpenType リガチャ | cosmic-text のシェーピング結果からリガチャを検出・描画 | sdit-core | 未着手 |

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
