# cosmic-text: グリフラスタライズのパターン

## 基本フロー

```
FontSystem::new()                   // システムフォント読み込み（重い）
  ↓
Buffer::new(&mut font_system, metrics)  // Metrics = font_size + line_height
  ↓
buf.set_text(&mut fs, text, Attrs::new(), Shaping::Advanced)
  ↓
buf.shape_until_scroll(&mut fs, false)  // シェーピング
  ↓
buf.layout_runs()                   // LayoutRunIter
  → run.glyphs[0].physical((0.0, 0.0), 1.0)  // PhysicalGlyph
  ↓
SwashCache::get_image_uncached(&mut fs, physical.cache_key)  // SwashImage
  → image.placement: Placement { left, top, width, height }
  → image.data: Vec<u8>  // R8 アルファマスク
```

## セル幅の計算

```rust
font.monospace_em_width()  // Option<f32>: em 単位 (0.0〜1.0)
// → em_width * font_size [px] = セル幅 [px]
// フォントが None の場合は font_size * 0.6 をフォールバック
```

## キャッシュキーの設計

`fontdb::ID` は SlotMap ベースで u32 に変換できない。
ハッシュキーには `fontdb::ID` と `glyph_id: u16` をそのまま使う。

```rust
#[derive(Hash, Eq, PartialEq, Clone)]
struct GlyphCacheKey {
    font_id: fontdb::ID,
    glyph_id: u16,
}
```

## 注意点

- `FontSystem::new()` はデバッグビルドで数秒かかる。アプリ起動時に1回だけ呼ぶ。
- `Buffer` はシェーピング毎に作り直しても問題ないが、重い場合はキャッシュする。
- `SwashCache::get_image_uncached` はキャッシュしない版。ラスタライズ後は atlas に書き込んで atlas レベルでキャッシュする。
- `image.content` が `Content::Mask` のみを想定（Alpha フォーマット）。カラー絵文字は別途対応が必要。
