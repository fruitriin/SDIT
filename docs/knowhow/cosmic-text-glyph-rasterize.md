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

## SwashContent の種類と RGBA 変換（Phase 10.3a で実装）

`image.content` には3種類あり、それぞれ異なる変換が必要。Atlas は `Rgba8Unorm`（4 bytes/pixel）を使用。

```rust
use cosmic_text::SwashContent;

let rgba_data: Vec<u8> = match image.content {
    SwashContent::Mask => {
        // グレースケール Alpha → RGBA: R=G=B=255, A=alpha
        image.data.iter().flat_map(|&a| [255u8, 255, 255, a]).collect()
    }
    SwashContent::Color => {
        // カラー絵文字: swash は BGRA 順で返す → RGBA に並び替え
        image.data.chunks_exact(4)
            .flat_map(|bgra| [bgra[2], bgra[1], bgra[0], bgra[3]])
            .collect()
    }
    SwashContent::SubpixelMask => {
        // サブピクセル: RGB → RGBA（A = max(R,G,B)）
        image.data.chunks_exact(3)
            .flat_map(|rgb| { let a = rgb[0].max(rgb[1]).max(rgb[2]); [rgb[0], rgb[1], rgb[2], a] })
            .collect()
    }
};
```

カラー絵文字かどうかは `GlyphEntry.is_color` フラグで保持し、シェーダーに渡す。
シェーダー側では `is_color_glyph > 0.5` の場合にテクスチャの RGBA をそのまま使用する（`mix(bg, texel, texel.a)`）。

## 注意点

- `FontSystem::new()` はデバッグビルドで数秒かかる。アプリ起動時に1回だけ呼ぶ。
- `Buffer` はシェーピング毎に作り直しても問題ないが、重い場合はキャッシュする。
- `SwashCache::get_image_uncached` はキャッシュしない版。ラスタライズ後は atlas に書き込んで atlas レベルでキャッシュする。
- `image.content` の種別チェックが必要。`SwashContent::Color` の場合は BGRA 順なので注意。
