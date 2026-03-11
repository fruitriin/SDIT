//! フォントコンテキスト — cosmic-text でグリフをラスタライズしアトラスに配置する。
//!
//! 実装方針:
//! - `FontSystem::new()` でシステムフォントを読み込む
//! - `Buffer` に1文字をセットしてシェーピング → `PhysicalGlyph` を得る
//! - `SwashCache` でラスタライズ → `Atlas` に配置

use std::collections::HashMap;

use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Placement, Shaping, SwashCache, fontdb};

use crate::atlas::{Atlas, AtlasRegion};

/// セルのピクセルメトリクス。
#[derive(Debug, Clone, Copy)]
pub struct CellMetrics {
    /// セル幅（ピクセル）。
    pub cell_width: f32,
    /// セル高さ（ピクセル）。
    pub cell_height: f32,
    /// セル上端からベースラインまで（ピクセル）。
    pub baseline: f32,
    /// フォントサイズ（ピクセル）。
    pub font_size: f32,
}

/// アトラスに配置されたグリフのエントリ。
#[derive(Debug, Clone)]
pub struct GlyphEntry {
    /// アトラス内の矩形領域。
    pub region: AtlasRegion,
    /// ベースラインからの X オフセット（ピクセル）。
    pub placement_left: i32,
    /// ベースラインからの Y オフセット（ピクセル）。正値 = 上方向。
    pub placement_top: i32,
}

/// グリフキャッシュのキー。
#[derive(Hash, Eq, PartialEq, Clone)]
struct GlyphCacheKey {
    /// フォント ID（fontdb）。
    font_id: fontdb::ID,
    /// グリフ ID。
    glyph_id: u16,
}

/// フォントコンテキスト。グリフのラスタライズとキャッシュを管理する。
pub struct FontContext {
    font_system: FontSystem,
    swash_cache: SwashCache,
    glyph_cache: HashMap<GlyphCacheKey, GlyphEntry>,
    metrics: CellMetrics,
    font_size: f32,
}

impl FontContext {
    /// 新しいフォントコンテキストを作成する。
    ///
    /// `font_size`: フォントサイズ（ピクセル）
    /// `line_height_factor`: 行の高さの倍率（例: 1.2）
    pub fn new(font_size: f32, line_height_factor: f32) -> Self {
        let mut font_system = FontSystem::new();
        let line_height = font_size * line_height_factor;

        // モノスペースフォントの em 幅からセル幅を計算する。
        let cell_width = compute_cell_width(&mut font_system, font_size);

        // ベースラインはフォントサイズの 80% 程度を目安にする（近似値）。
        // より正確な値は Buffer の shaping 結果から得られるが、初期実装では固定値を使う。
        let baseline = font_size * 0.8;

        let metrics = CellMetrics { cell_width, cell_height: line_height, baseline, font_size };

        Self {
            font_system,
            swash_cache: SwashCache::new(),
            glyph_cache: HashMap::new(),
            metrics,
            font_size,
        }
    }

    /// セルメトリクスを返す。
    pub fn metrics(&self) -> &CellMetrics {
        &self.metrics
    }

    /// 文字 `c` をラスタライズしてアトラスに配置し、`GlyphEntry` を返す。
    ///
    /// キャッシュ済みの場合はキャッシュを返す。スペースや描画不要な文字は `None`。
    pub fn rasterize_glyph(&mut self, c: char, atlas: &mut Atlas) -> Option<&GlyphEntry> {
        // スペースは描画不要。
        if c == ' ' || c == '\0' {
            return None;
        }

        // Buffer でシェーピングして PhysicalGlyph を得る。
        let line_height = self.font_size * 1.2;
        let metrics = Metrics::new(self.font_size, line_height);
        let mut buf = Buffer::new(&mut self.font_system, metrics);
        let mut s = [0u8; 4];
        let text = c.encode_utf8(&mut s);
        buf.set_text(&mut self.font_system, text, Attrs::new(), Shaping::Advanced);
        buf.shape_until_scroll(&mut self.font_system, false);

        // グリフ情報を取り出す。
        let physical = buf
            .layout_runs()
            .next()
            .and_then(|run| run.glyphs.first())
            .map(|glyph| glyph.physical((0.0, 0.0), 1.0))?;

        let cache_key_raw = GlyphCacheKey {
            font_id: physical.cache_key.font_id,
            glyph_id: physical.cache_key.glyph_id,
        };

        // キャッシュ済みなら返す。
        if self.glyph_cache.contains_key(&cache_key_raw) {
            return self.glyph_cache.get(&cache_key_raw);
        }

        // ラスタライズ。
        let image =
            self.swash_cache.get_image_uncached(&mut self.font_system, physical.cache_key)?;

        let placement: Placement = image.placement;
        let w = placement.width;
        let h = placement.height;
        if w == 0 || h == 0 {
            return None;
        }

        // アトラスに確保して書き込む。
        let region = atlas.reserve(w, h)?;
        atlas.write(region, &image.data);

        let entry =
            GlyphEntry { region, placement_left: placement.left, placement_top: placement.top };
        self.glyph_cache.insert(cache_key_raw.clone(), entry);
        self.glyph_cache.get(&cache_key_raw)
    }
}

/// モノスペースフォントの em 幅からセル幅を計算する。
/// フォントが見つからない場合は `font_size * 0.6` をフォールバックとして使う。
fn compute_cell_width(font_system: &mut FontSystem, font_size: f32) -> f32 {
    // モノスペースフォントの最初の ID を探す。
    let mono_id = font_system.db().faces().find(|f| f.monospaced).map(|f| f.id);

    if let Some(id) = mono_id {
        if let Some(font) = font_system.get_font(id) {
            if let Some(em_width) = font.monospace_em_width() {
                // em_width は em 単位（0〜1）。font_size px を乗じてピクセルに変換。
                let width = em_width * font_size;
                if width > 0.0 {
                    return width;
                }
            }
        }
    }

    // フォールバック: フォントサイズの 60%。
    font_size * 0.6
}

// ---------------------------------------------------------------------------
// テスト
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_metrics_positive() {
        let ctx = FontContext::new(14.0, 1.2);
        let m = ctx.metrics();
        assert!(m.cell_width > 0.0, "cell_width must be positive");
        assert!(m.cell_height > 0.0, "cell_height must be positive");
        assert!(m.baseline > 0.0, "baseline must be positive");
        assert!((m.font_size - 14.0).abs() < f32::EPSILON);
    }

    #[test]
    fn cell_height_matches_line_height() {
        let font_size = 16.0_f32;
        let factor = 1.2_f32;
        let ctx = FontContext::new(font_size, factor);
        let expected = font_size * factor;
        // 小数点誤差を考慮して epsilon を設ける。
        assert!(
            (ctx.metrics().cell_height - expected).abs() < 0.01,
            "cell_height = {}, expected = {}",
            ctx.metrics().cell_height,
            expected
        );
    }
}
