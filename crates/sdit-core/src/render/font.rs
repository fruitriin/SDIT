//! フォントコンテキスト — cosmic-text でグリフをラスタライズしアトラスに配置する。
//!
//! 実装方針:
//! - `FontSystem::new()` でシステムフォントを読み込む
//! - `Buffer` に1文字をセットしてシェーピング → `PhysicalGlyph` を得る
//! - `SwashCache` でラスタライズ → `Atlas` に配置

use std::collections::HashMap;

use crate::config::font::FontConfig;
use cosmic_text::{
    Attrs, Buffer, Family, FontSystem, Metrics, Placement, Shaping, SwashCache, fontdb,
};

use super::atlas::{Atlas, AtlasRegion};

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
    /// 行の高さの倍率（例: 1.2 = フォントサイズの 120%）。
    line_height_factor: f32,
    /// シェーピング時に使うフォントファミリ名。
    font_family: String,
}

impl FontContext {
    /// 新しいフォントコンテキストを作成する。
    ///
    /// `font_size`: フォントサイズ（ピクセル）
    /// `line_height_factor`: 行の高さの倍率（例: 1.2）
    pub fn new(font_size: f32, line_height_factor: f32) -> Self {
        Self::from_config(&FontConfig {
            size: font_size,
            line_height: line_height_factor,
            ..FontConfig::default()
        })
    }

    /// `FontConfig` からフォントコンテキストを作成する。
    pub fn from_config(config: &FontConfig) -> Self {
        let mut font_system = FontSystem::new();
        let font_size = config.clamped_size();
        let line_height = font_size * config.clamped_line_height();

        // モノスペースフォントの em 幅からセル幅を計算する。
        let cell_width = compute_cell_width(&mut font_system, font_size);

        // ベースラインはフォントサイズの 80% 程度を目安にする（近似値）。
        let baseline = font_size * 0.8;

        let metrics = CellMetrics { cell_width, cell_height: line_height, baseline, font_size };

        Self {
            font_system,
            swash_cache: SwashCache::new(),
            glyph_cache: HashMap::new(),
            metrics,
            font_size,
            line_height_factor: config.clamped_line_height(),
            font_family: config.family.clone(),
        }
    }

    /// セルメトリクスを返す。
    pub fn metrics(&self) -> &CellMetrics {
        &self.metrics
    }

    /// フォントサイズを変更してメトリクスを再計算する。グリフキャッシュはクリアされる。
    ///
    /// アトラス側も別途 `Atlas::clear()` を呼ぶこと。
    ///
    /// `font_size` は 1.0〜200.0 の範囲にクランプされる。
    pub fn set_font_size(&mut self, font_size: f32) {
        let font_size = if font_size.is_finite() { font_size.clamp(1.0, 200.0) } else { 14.0 };
        self.font_size = font_size;
        let line_height = font_size * self.line_height_factor;
        let cell_width = compute_cell_width(&mut self.font_system, font_size);
        let baseline = font_size * 0.8;
        self.metrics = CellMetrics { cell_width, cell_height: line_height, baseline, font_size };
        self.glyph_cache.clear();
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
        let attrs = Attrs::new().family(Family::Name(&self.font_family));
        buf.set_text(&mut self.font_system, text, attrs, Shaping::Advanced);
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
    fn set_font_size_updates_metrics() {
        let mut ctx = FontContext::new(14.0, 1.2);
        ctx.set_font_size(20.0);
        assert!(
            (ctx.metrics().font_size - 20.0).abs() < f32::EPSILON,
            "font_size should be 20.0, got {}",
            ctx.metrics().font_size
        );
        assert!(ctx.metrics().cell_width > 0.0, "cell_width must be positive after resize");
        assert!(ctx.metrics().cell_height > 0.0, "cell_height must be positive after resize");
    }

    #[test]
    fn set_font_size_clamps_to_bounds() {
        let mut ctx = FontContext::new(14.0, 1.2);
        ctx.set_font_size(0.1);
        assert!(
            ctx.metrics().font_size >= 1.0,
            "font_size should be clamped to >= 1.0, got {}",
            ctx.metrics().font_size
        );
        ctx.set_font_size(999.0);
        assert!(
            ctx.metrics().font_size <= 200.0,
            "font_size should be clamped to <= 200.0, got {}",
            ctx.metrics().font_size
        );
    }

    #[test]
    fn set_font_size_preserves_line_height_factor() {
        let factor = 1.5_f32;
        let mut ctx = FontContext::new(14.0, factor);
        ctx.set_font_size(20.0);
        let expected_height = 20.0 * factor;
        assert!(
            (ctx.metrics().cell_height - expected_height).abs() < 0.01,
            "cell_height should be {} after set_font_size, got {}",
            expected_height,
            ctx.metrics().cell_height
        );
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
