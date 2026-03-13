# cosmic-text 0.12.1: LayoutGlyph API の注意点

## glyph.metadata はバイトオフセットではない

`LayoutGlyph.metadata` は `Attrs` から引き継いだユーザーメタデータ（デフォルト0）であり、
テキスト内のバイト位置ではない。

### 正しい API

| フィールド | 意味 |
|---|---|
| `glyph.start` | クラスターの開始バイトオフセット（入力テキスト内） |
| `glyph.end` | クラスターの終了バイトオフセット（入力テキスト内） |
| `glyph.metadata` | `Attrs::metadata` のコピー（ユーザー定義値、デフォルト0） |

### Buffer::set_size() は必須

`Buffer::new()` 直後に `set_size()` を呼ばないと `shape_until_scroll()` がレイアウトを実行しない。
折り返しなしの場合は幅に `f32::MAX` を使う。

```rust
let mut buf = Buffer::new(&mut font_system, metrics);
buf.set_size(&mut font_system, Some(f32::MAX), Some(line_height * 2.0));
buf.set_text(&mut font_system, text, attrs, Shaping::Advanced);
buf.shape_until_scroll(&mut font_system, false);
```

### glyph.start/end の安全性

- HarfBuzz の `char_indices()` 経由で設定されるため、UTF-8 バイト境界が保証される
- `end_run` は unicode_bidi 由来で同様に UTF-8 境界
- ターミナル行テキスト（改行なし）では `glyph.end > line_text.len()` にならない

## 発見経緯

Phase 10.3b で `shape_line()` を実装した際に `glyph.metadata` をバイトオフセットとして使用し、
全グリフの covered_text が空文字列となりテキストが表示されないバグが発生した（2026-03-13 修正）。
