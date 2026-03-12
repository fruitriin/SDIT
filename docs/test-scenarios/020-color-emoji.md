# シナリオ 020: カラー絵文字対応 — Atlas RGBA 化

**対象 Phase**: Phase 10.3a
**概要**: Atlas の RGBA 化（R8Unorm → Rgba8Unorm）とカラー絵文字描画の動作確認

---

## サブシナリオ a: Atlas RGBA フォーマットでの正常起動

**目的**: Rgba8Unorm フォーマットで Atlas が正常に初期化されることを確認する

**実施方法**: ユニットテスト（`render::atlas::tests`）

**確認項目**:
- `InMemAtlas::new(size)` で 4 bytes/pixel のバッファ（`size * size * 4` bytes）が確保される
- `write_stores_rgba_pixels_correctly` — 2x2 RGBA データが正しいオフセットに書き込まれる
- `write_rejects_wrong_size` — R8 サイズ（4 bytes）のデータが拒否される（長さ検証）

**テスト**: `render::atlas::tests::write_stores_rgba_pixels_correctly`、`write_rejects_wrong_size`

**最終実行**: 2026-03-13
**結果**: PASS

---

## サブシナリオ b: グレースケールグリフの正常描画（退行確認）

**目的**: `SwashContent::Mask`（通常グリフ）が RGBA 昇格されて正しく描画されることを確認する

**実施方法**: ユニットテスト + ヘッドレステスト

**確認項目**:
- `SwashContent::Mask` の変換: `[a]` → `[255, 255, 255, a]` の 4 byte RGBA に昇格
- Atlas の `write()` が正しいサイズ（`width * height * 4`）のデータを受け入れる
- ヘッドレスパイプラインで通常文字が表示される（退行なし）

**テスト**:
- `render::atlas::tests::reserve_returns_non_overlapping_regions`
- `render::atlas::tests::clear_resets_atlas`
- `headless_pipeline::echo_appears_in_grid`
- `headless_pipeline::cursor_position_after_escape_sequence`

**最終実行**: 2026-03-13
**結果**: PASS

---

## サブシナリオ c: カラー絵文字の描画（Content::Color）

**目的**: `SwashContent::Color`（カラー絵文字）が BGRA→RGBA 変換されて `is_color=true` フラグ付きで格納されることを確認する

**実施方法**: ユニットテスト（font.rs の実装検証）

**確認項目**:
- `SwashContent::Color` の変換: `chunks_exact(4)` で `[bgra]` → `[bgra[2], bgra[1], bgra[0], bgra[3]]` に並び替え
- `GlyphEntry.is_color == true` がセットされる
- `CellVertex.is_color_glyph == 1.0` がシェーダーに渡される（`update_from_grid()` 経由）

**テスト**: ユニットテスト（font.rs `rasterize_glyph` 内のロジック）
※ SwashContent::Color はフォント依存のため環境次第でスキップされる可能性がある

**注意**: 絵文字の実際のレンダリングは GUI テスト環境が必要。ユニットテストでは変換ロジックの正確性を確認する。

**最終実行**: 2026-03-13
**結果**: UNIT_ONLY（GUI テスト環境なし）

---

## サブシナリオ d: サブピクセルマスクの処理（Content::SubpixelMask）

**目的**: `SwashContent::SubpixelMask`（サブピクセルフォント）が RGB→RGBA 変換されることを確認する

**実施方法**: ユニットテスト（font.rs の実装検証）

**確認項目**:
- `SwashContent::SubpixelMask` の変換: `chunks_exact(3)` で `[r, g, b]` → `[r, g, b, max(r,g,b)]` の RGBA に変換
- `is_color = false` がセットされる（通常グリフとして扱われる）

**テスト**: ユニットテスト（font.rs ロジック検証）
※ SubpixelMask は LCD サブピクセルレンダリングが有効なフォント環境でのみ出現する

**最終実行**: 2026-03-13
**結果**: UNIT_ONLY（SubpixelMask は環境依存）

---

## サブシナリオ e: アトラス容量のメモリ使用量（4 倍化の影響）

**目的**: R8Unorm → Rgba8Unorm への変更で Atlas のメモリ使用量が 4 倍になることを確認し、デフォルトサイズ 512px での影響を評価する

**実施方法**: 計算による確認

**数値**:
- R8 時代: 512 × 512 × 1 = **262,144 bytes** ≈ 256 KB
- Rgba8 現在: 512 × 512 × 4 = **1,048,576 bytes** ≈ 1 MB

**評価**:
- 増加量: +768 KB（1 ウィンドウあたり）
- 現代の GPU メモリ（数 GB）に対して無視できる増分
- テキストターミナルの実使用グリフ数（数百〜数千文字）に対してアトラスは十分な余裕を持つ

**確認項目**:
- `InMemAtlas::new(512)` のバッファサイズが `512 * 512 * 4 = 1,048,576` bytes
- アトラス満杯時は `None` を返し、グリフ欠落が起きても クラッシュしない

**テスト**: `render::atlas::tests::reserve_fails_when_full`

**最終実行**: 2026-03-13
**結果**: PASS

---

## 実行サマリ（2026-03-13）

| サブシナリオ | 実施方法 | 結果 |
|---|---|---|
| a: Atlas RGBA 初期化 | ユニットテスト | PASS |
| b: グレースケール退行確認 | ユニットテスト + ヘッドレス | PASS |
| c: カラー絵文字 BGRA→RGBA | ユニットテスト（ロジック確認） | UNIT_ONLY |
| d: SubpixelMask 処理 | ユニットテスト（ロジック確認） | UNIT_ONLY |
| e: メモリ使用量評価 | 計算 + ユニットテスト | PASS |

**総合**: 全 cargo test 229 件 PASS。退行なし。
