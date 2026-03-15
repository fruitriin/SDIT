# グリフキャッシュと Atlas の不整合（マルチウィンドウ）

## 発見日
2026-03-15

## 問題
Cmd+N で新ウィンドウを開くと、フォントがレンダリングされない（空白）。
テキスト自体は内在しており、選択→コピーは可能。

## 根本原因
`FontContext` はアプリ全体で 1 つ共有されており、`glyph_cache: HashMap<GlyphCacheKey, GlyphEntry>` にラスタライズ済みグリフの Atlas リージョン情報をキャッシュしている。

一方 `Atlas` はウィンドウ（GPU デバイス）ごとに独立して作成される。

1. ウィンドウ1: `rasterize_glyph('A', &mut atlas1)` → atlas1 にピクセルデータ書き込み → `glyph_cache` に `{ region: atlas1の位置 }` を保存
2. ウィンドウ2: `rasterize_glyph('A', &mut atlas2)` → **キャッシュヒット** → atlas1 の位置情報を返す
3. atlas2 にはピクセルデータが存在しない → 空白で描画

## 解決策
新しい Atlas を作成する箇所で `font_ctx.clear_glyph_cache()` を呼び、キャッシュを無効化する。
次回の `redraw_session` で全グリフが新しい Atlas に再ラスタライズされる。

```rust
self.font_ctx.clear_glyph_cache();
let mut atlas = Atlas::new(&gpu.device, 512);
```

対象箇所（Atlas::new の全呼び出し元）:
- `create_window_with_cwd` — 通常の新ウィンドウ
- Quick Terminal 初期化
- `detach_session_to_new_window` — タブ切り出し

## 副作用
キャッシュクリアにより既存ウィンドウのキャッシュも無効化されるが、次回 `redraw_session` で再ラスタライズされるため実害はない（一瞬のちらつきが起きる可能性はある）。

## より良い設計（将来）
- Atlas ごとにグリフキャッシュを持つ（`Atlas` 内にキャッシュを統合）
- または `GlyphEntry` に Atlas の識別子を含め、異なる Atlas のキャッシュを区別する
