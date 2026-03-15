# グリフキャッシュと Atlas の不整合（マルチウィンドウ）

## 発見日
2026-03-15

## 問題
Cmd+N で新ウィンドウを開く、またはタブをドラッグで切り出すと、フォントがレンダリングされない（空白）。
テキスト自体は内在しており、選択→コピーは可能。

## 根本原因
`FontContext` はアプリ全体で 1 つ共有されており、`glyph_cache: HashMap<GlyphCacheKey, GlyphEntry>` にラスタライズ済みグリフの Atlas リージョン情報をキャッシュしている。

一方 `Atlas` はウィンドウ（GPU デバイス）ごとに独立して作成される。

キャッシュキーが `(font_id, glyph_id)` のみだった場合:
1. ウィンドウ1: `rasterize_glyph('A', &mut atlas1)` → atlas1 にピクセルデータ書き込み → キャッシュに `{ region: atlas1の位置 }` を保存
2. ウィンドウ2: `rasterize_glyph('A', &mut atlas2)` → **キャッシュヒット** → atlas1 の位置情報を返す
3. atlas2 にはピクセルデータが存在しない → 空白で描画

### 初期修正の失敗: `clear_glyph_cache` 方式

最初は `Atlas::new` の前に `font_ctx.clear_glyph_cache()` でキャッシュ全体をクリアする方式で修正した。
これは単一ウィンドウ→新ウィンドウのケースでは動作したが、**マルチウィンドウ環境で致命的な問題** を引き起こした:

- `about_to_wait` でのフレームスロットリングにより、複数ウィンドウが交互に `redraw_session` される
- 一方のウィンドウの再描画でキャッシュが汚染され、もう一方のウィンドウで一部のグリフだけが表示される挙動不審な状態になった
- `dirty_sessions` に追加して再描画を保証する試みも、キャッシュの相互汚染を悪化させた

## 解決策（最終版）
`Atlas` に一意の `AtlasId`（`u64`、アトミックカウンタで生成）を追加し、`GlyphCacheKey` に `atlas_id` フィールドを含める:

```rust
// atlas.rs
pub type AtlasId = u64;

pub struct Atlas {
    id: AtlasId,
    // ...
}

// font.rs
struct GlyphCacheKey {
    font_id: fontdb::ID,
    glyph_id: u16,
    atlas_id: AtlasId,  // 追加
}
```

これにより:
- 同じグリフでも Atlas が異なれば別のキャッシュエントリになる
- 各ウィンドウの Atlas に対して独立してラスタライズ＋キャッシュされる
- `clear_glyph_cache()` は不要（呼び出し箇所を全て除去）
- マルチウィンドウ環境でも安全

## 副作用
- 同じグリフが複数の Atlas に重複してラスタライズされる（メモリ使用量が微増）
- キャッシュエントリが Atlas 数 × グリフ数に増える
- ウィンドウを閉じても古い Atlas ID のキャッシュエントリが残る（`set_font_size` 時に全クリアされる）

## 教訓
- 共有キャッシュ + ウィンドウ固有リソースの組み合わせは、キャッシュキーにリソースの識別子を含めないと不整合が起きる
- `clear_cache` による全クリアは単純なケースでは動くが、並行・交互アクセスパターンで破綻する
- アトミックカウンタによる ID 生成は安価で確実
