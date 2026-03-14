use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// コードポイント範囲に対応するフォントファミリ指定。
///
/// 設定の `codepoint_map` を validate 時にパースして保持する。
#[derive(Debug, Clone, PartialEq)]
pub struct CodepointRange {
    /// Unicode コードポイントの開始値（inclusive）。
    pub start: u32,
    /// Unicode コードポイントの終了値（inclusive）。
    pub end: u32,
    /// このレンジに適用するフォントファミリ名。
    pub family: String,
}

impl CodepointRange {
    /// "U+3000-U+9FFF" 形式の文字列をパースする。
    ///
    /// "U+3000" のように単一コードポイント指定も受け付ける（start == end）。
    /// パース失敗は `None` を返す。
    pub fn parse(range_str: &str, family: &str) -> Option<Self> {
        let range_str = range_str.trim();
        let parts: Vec<&str> = range_str.splitn(2, '-').collect();
        let start = parse_codepoint(parts[0])?;
        let end = if parts.len() == 2 { parse_codepoint(parts[1])? } else { start };
        if start > end {
            return None;
        }
        Some(CodepointRange { start, end, family: family.to_owned() })
    }

    /// 文字 `c` がこのレンジに含まれるかどうかを返す。
    pub fn contains(&self, c: char) -> bool {
        let cp = c as u32;
        cp >= self.start && cp <= self.end
    }
}

/// "U+3000" または "3000" 形式のコードポイント文字列を u32 にパースする。
fn parse_codepoint(s: &str) -> Option<u32> {
    let s = s.trim();
    let hex = if let Some(stripped) = s.strip_prefix("U+") {
        stripped
    } else if let Some(stripped) = s.strip_prefix("u+") {
        stripped
    } else {
        s
    };
    u32::from_str_radix(hex, 16).ok().filter(|&cp| cp <= 0x10FFFF)
}

/// セルサイズ・ベースライン調整値。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct FontAdjust {
    /// セル幅の加算値（ピクセル）。クランプ: -10.0〜10.0。デフォルト: 0.0。
    pub cell_width: f32,
    /// セル高さの加算値（ピクセル）。クランプ: -10.0〜10.0。デフォルト: 0.0。
    pub cell_height: f32,
    /// ベースラインの加算値（ピクセル）。クランプ: -20.0〜20.0。デフォルト: 0.0。
    pub baseline: f32,
}

impl Default for FontAdjust {
    fn default() -> Self {
        Self { cell_width: 0.0, cell_height: 0.0, baseline: 0.0 }
    }
}

impl FontAdjust {
    /// `cell_width` を安全な範囲にクランプする。NaN/Inf は 0.0 にフォールバック。
    pub fn clamped_cell_width(&self) -> f32 {
        if self.cell_width.is_finite() { self.cell_width.clamp(-10.0, 10.0) } else { 0.0 }
    }

    /// `cell_height` を安全な範囲にクランプする。NaN/Inf は 0.0 にフォールバック。
    pub fn clamped_cell_height(&self) -> f32 {
        if self.cell_height.is_finite() { self.cell_height.clamp(-10.0, 10.0) } else { 0.0 }
    }

    /// `baseline` を安全な範囲にクランプする。NaN/Inf は 0.0 にフォールバック。
    pub fn clamped_baseline(&self) -> f32 {
        if self.baseline.is_finite() { self.baseline.clamp(-20.0, 20.0) } else { 0.0 }
    }
}

/// フォント設定。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct FontConfig {
    /// フォントファミリ名。
    pub family: String,
    /// フォントサイズ（ピクセル）。
    pub size: f32,
    /// 行の高さの倍率（例: 1.2 = フォントサイズの 120%）。
    pub line_height: f32,
    /// フォールバックフォントファミリ名（CJK等）。
    #[serde(default)]
    pub fallback_families: Vec<String>,
    /// コードポイントレンジ別フォント指定。
    ///
    /// キー: "U+3000-U+9FFF" 形式のレンジ文字列（最大 64 エントリ）。
    /// 値: フォントファミリ名（最大 128 文字）。
    ///
    /// 例: `{ "U+3000-U+9FFF" = "Noto Sans CJK" }`
    #[serde(default)]
    pub codepoint_map: HashMap<String, String>,
    /// OpenType フォントバリエーション設定（最大 16 エントリ）。
    ///
    /// キー: 4文字の OpenType タグ（例: "wght", "wdth"）（最大 8 文字）。
    /// 値: バリエーション値（例: 700.0）。
    ///
    /// 注意: cosmic-text v0.12 では実行時 variation 適用の API がないため、
    /// 設定の読み込み・保存のみサポートし、将来バージョンで適用する。
    #[serde(default)]
    pub variation: HashMap<String, f32>,
    /// OpenType フォントフィーチャー設定（最大 32 エントリ）。
    ///
    /// キー: 4文字の OpenType タグ（例: "calt", "liga"）（最大 8 文字）。
    /// 値: true = 有効、false = 無効。
    ///
    /// 注意: cosmic-text v0.12 では実行時 feature 適用の API がないため、
    /// 設定の読み込み・保存のみサポートし、将来バージョンで適用する。
    #[serde(default)]
    pub feature: HashMap<String, bool>,
    /// セルサイズ・ベースライン調整値。
    #[serde(default)]
    pub adjust: FontAdjust,
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            family: default_font_family().to_owned(),
            size: 14.0,
            line_height: 1.2,
            fallback_families: Vec::new(),
            codepoint_map: HashMap::new(),
            variation: HashMap::new(),
            feature: HashMap::new(),
            adjust: FontAdjust::default(),
        }
    }
}

/// プラットフォーム別のデフォルトフォントファミリ。
fn default_font_family() -> &'static str {
    if cfg!(target_os = "macos") {
        "Menlo"
    } else if cfg!(target_os = "windows") {
        "Consolas"
    } else {
        "monospace"
    }
}

impl FontConfig {
    /// フォントサイズを安全な範囲にクランプする（1.0〜200.0）。
    /// NaN/Infinity はデフォルト値（14.0）にフォールバックする。
    pub fn clamped_size(&self) -> f32 {
        if self.size.is_finite() { self.size.clamp(1.0, 200.0) } else { 14.0 }
    }

    /// 行高倍率を安全な範囲にクランプする（0.5〜5.0）。
    /// NaN/Infinity はデフォルト値（1.2）にフォールバックする。
    pub fn clamped_line_height(&self) -> f32 {
        if self.line_height.is_finite() { self.line_height.clamp(0.5, 5.0) } else { 1.2 }
    }

    /// `codepoint_map` をパースして `Vec<CodepointRange>` に変換する。
    ///
    /// パース失敗エントリはスキップする。エントリ数は最大 64 に制限する。
    pub fn parsed_codepoint_map(&self) -> Vec<CodepointRange> {
        self.codepoint_map
            .iter()
            .take(64)
            .filter_map(|(range_str, family)| {
                // キー長チェック（32文字以内）
                if range_str.len() > 32 {
                    return None;
                }
                // 値長チェック（128文字以内）
                if family.len() > 128 {
                    return None;
                }
                CodepointRange::parse(range_str, family)
            })
            .collect()
    }

    /// `variation` のエントリ数を最大 16 に制限して返す。NaN/Inf 値は除外する。
    pub fn clamped_variation(&self) -> impl Iterator<Item = (&str, f32)> {
        self.variation
            .iter()
            .take(16)
            .filter(|(k, _)| k.len() <= 8)
            .filter_map(|(k, &v)| if v.is_finite() { Some((k.as_str(), v)) } else { None })
    }

    /// デシリアライズ後に各 HashMap のエントリ数を上限に合わせて truncate する。
    ///
    /// - `codepoint_map`: 最大 64 エントリ
    /// - `variation`: 最大 16 エントリ
    /// - `feature`: 最大 32 エントリ
    pub fn validate(&mut self) {
        if self.codepoint_map.len() > 64 {
            log::warn!(
                "Too many codepoint_map entries ({}), truncating to 64",
                self.codepoint_map.len()
            );
            let keys: Vec<_> = self.codepoint_map.keys().skip(64).cloned().collect();
            for k in keys {
                self.codepoint_map.remove(&k);
            }
        }
        if self.variation.len() > 16 {
            log::warn!("Too many variation entries ({}), truncating to 16", self.variation.len());
            let keys: Vec<_> = self.variation.keys().skip(16).cloned().collect();
            for k in keys {
                self.variation.remove(&k);
            }
        }
        if self.feature.len() > 32 {
            log::warn!("Too many feature entries ({}), truncating to 32", self.feature.len());
            let keys: Vec<_> = self.feature.keys().skip(32).cloned().collect();
            for k in keys {
                self.feature.remove(&k);
            }
        }
    }

    /// `feature` のエントリ数を最大 32 に制限して返す。
    pub fn clamped_feature(&self) -> impl Iterator<Item = (&str, bool)> {
        self.feature.iter().take(32).filter(|(k, _)| k.len() <= 8).map(|(k, &v)| (k.as_str(), v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_font_config() {
        let config = FontConfig::default();
        assert!(!config.family.is_empty());
        assert!((config.size - 14.0).abs() < f32::EPSILON);
        assert!((config.line_height - 1.2).abs() < f32::EPSILON);
    }

    #[test]
    fn clamped_size_bounds() {
        let small = FontConfig { size: 0.0, ..FontConfig::default() };
        assert!((small.clamped_size() - 1.0).abs() < f32::EPSILON);
        let large = FontConfig { size: 999.0, ..FontConfig::default() };
        assert!((large.clamped_size() - 200.0).abs() < f32::EPSILON);
    }

    #[test]
    fn clamped_nan_fallback() {
        let nan = FontConfig { size: f32::NAN, line_height: f32::NAN, ..FontConfig::default() };
        assert!((nan.clamped_size() - 14.0).abs() < f32::EPSILON);
        assert!((nan.clamped_line_height() - 1.2).abs() < f32::EPSILON);

        let inf = FontConfig {
            size: f32::INFINITY,
            line_height: f32::NEG_INFINITY,
            ..FontConfig::default()
        };
        assert!((inf.clamped_size() - 14.0).abs() < f32::EPSILON);
        assert!((inf.clamped_line_height() - 1.2).abs() < f32::EPSILON);
    }

    #[test]
    fn deserialize_partial() {
        let toml_str = "size = 18.0";
        let config: FontConfig = toml::from_str(toml_str).unwrap();
        assert!((config.size - 18.0).abs() < f32::EPSILON);
        // family と line_height はデフォルト値
        assert!(!config.family.is_empty());
        assert!((config.line_height - 1.2).abs() < f32::EPSILON);
    }

    // -----------------------------------------------------------------------
    // FontAdjust テスト
    // -----------------------------------------------------------------------

    #[test]
    fn font_adjust_default_values() {
        let adj = FontAdjust::default();
        assert!((adj.cell_width - 0.0).abs() < f32::EPSILON);
        assert!((adj.cell_height - 0.0).abs() < f32::EPSILON);
        assert!((adj.baseline - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn font_adjust_clamp_cell_width() {
        let adj = FontAdjust { cell_width: 20.0, ..Default::default() };
        assert!((adj.clamped_cell_width() - 10.0).abs() < f32::EPSILON);
        let adj = FontAdjust { cell_width: -20.0, ..Default::default() };
        assert!((adj.clamped_cell_width() - (-10.0)).abs() < f32::EPSILON);
        let adj = FontAdjust { cell_width: 3.5, ..Default::default() };
        assert!((adj.clamped_cell_width() - 3.5).abs() < f32::EPSILON);
    }

    #[test]
    fn font_adjust_clamp_cell_height() {
        let adj = FontAdjust { cell_height: 15.0, ..Default::default() };
        assert!((adj.clamped_cell_height() - 10.0).abs() < f32::EPSILON);
        let adj = FontAdjust { cell_height: -15.0, ..Default::default() };
        assert!((adj.clamped_cell_height() - (-10.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn font_adjust_clamp_baseline() {
        let adj = FontAdjust { baseline: 30.0, ..Default::default() };
        assert!((adj.clamped_baseline() - 20.0).abs() < f32::EPSILON);
        let adj = FontAdjust { baseline: -30.0, ..Default::default() };
        assert!((adj.clamped_baseline() - (-20.0)).abs() < f32::EPSILON);
        let adj = FontAdjust { baseline: 5.0, ..Default::default() };
        assert!((adj.clamped_baseline() - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn font_adjust_nan_fallback() {
        let adj = FontAdjust {
            cell_width: f32::NAN,
            cell_height: f32::INFINITY,
            baseline: f32::NEG_INFINITY,
        };
        assert!((adj.clamped_cell_width() - 0.0).abs() < f32::EPSILON);
        assert!((adj.clamped_cell_height() - 0.0).abs() < f32::EPSILON);
        assert!((adj.clamped_baseline() - 0.0).abs() < f32::EPSILON);
    }

    // -----------------------------------------------------------------------
    // CodepointRange テスト
    // -----------------------------------------------------------------------

    #[test]
    fn codepoint_range_parse_range() {
        let r = CodepointRange::parse("U+3000-U+9FFF", "Noto Sans CJK").unwrap();
        assert_eq!(r.start, 0x3000);
        assert_eq!(r.end, 0x9FFF);
        assert_eq!(r.family, "Noto Sans CJK");
    }

    #[test]
    fn codepoint_range_parse_single() {
        let r = CodepointRange::parse("U+3042", "Hiragino").unwrap();
        assert_eq!(r.start, 0x3042);
        assert_eq!(r.end, 0x3042);
    }

    #[test]
    fn codepoint_range_parse_lowercase_u() {
        let r = CodepointRange::parse("u+0041-u+005A", "Arial").unwrap();
        assert_eq!(r.start, 0x0041); // 'A'
        assert_eq!(r.end, 0x005A); // 'Z'
    }

    #[test]
    fn codepoint_range_parse_without_u_prefix() {
        // プレフィックスなしの hex も許容する
        let r = CodepointRange::parse("3000-9FFF", "Fallback").unwrap();
        assert_eq!(r.start, 0x3000);
        assert_eq!(r.end, 0x9FFF);
    }

    #[test]
    fn codepoint_range_parse_invalid_returns_none() {
        assert!(CodepointRange::parse("INVALID", "Font").is_none());
        assert!(CodepointRange::parse("U+ZZZZ", "Font").is_none());
        // start > end は無効
        assert!(CodepointRange::parse("U+9FFF-U+3000", "Font").is_none());
    }

    #[test]
    fn codepoint_range_contains() {
        let r = CodepointRange::parse("U+3040-U+309F", "Hiragino").unwrap();
        assert!(r.contains('\u{3042}')); // あ
        assert!(r.contains('\u{3040}'));
        assert!(r.contains('\u{309F}'));
        assert!(!r.contains('A'));
    }

    // -----------------------------------------------------------------------
    // codepoint_map パース・バリデーションテスト
    // -----------------------------------------------------------------------

    #[test]
    fn parsed_codepoint_map_basic() {
        let mut config = FontConfig::default();
        config.codepoint_map.insert("U+3000-U+9FFF".to_owned(), "Noto CJK".to_owned());
        let ranges = config.parsed_codepoint_map();
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].start, 0x3000);
        assert_eq!(ranges[0].family, "Noto CJK");
    }

    #[test]
    fn parsed_codepoint_map_invalid_key_skipped() {
        let mut config = FontConfig::default();
        config.codepoint_map.insert("INVALID".to_owned(), "Font".to_owned());
        let ranges = config.parsed_codepoint_map();
        assert!(ranges.is_empty());
    }

    #[test]
    fn parsed_codepoint_map_key_too_long_skipped() {
        let mut config = FontConfig::default();
        let long_key = "U+0041-U+".to_owned() + &"F".repeat(30);
        config.codepoint_map.insert(long_key, "Font".to_owned());
        let ranges = config.parsed_codepoint_map();
        assert!(ranges.is_empty());
    }

    #[test]
    fn parsed_codepoint_map_value_too_long_skipped() {
        let mut config = FontConfig::default();
        config.codepoint_map.insert("U+3000-U+9FFF".to_owned(), "F".repeat(129));
        let ranges = config.parsed_codepoint_map();
        assert!(ranges.is_empty());
    }

    // -----------------------------------------------------------------------
    // variation / feature デシリアライズテスト
    // -----------------------------------------------------------------------

    #[test]
    fn deserialize_variation() {
        let toml_str = r#"
[font]
family = "JetBrains Mono"

[font.variation]
wght = 700.0
wdth = 100.0
"#;
        let config: crate::config::Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.font.variation.get("wght"), Some(&700.0_f32));
        assert_eq!(config.font.variation.get("wdth"), Some(&100.0_f32));
    }

    #[test]
    fn deserialize_feature() {
        let toml_str = r#"
[font]
family = "FiraCode"

[font.feature]
calt = true
liga = false
"#;
        let config: crate::config::Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.font.feature.get("calt"), Some(&true));
        assert_eq!(config.font.feature.get("liga"), Some(&false));
    }

    #[test]
    fn deserialize_adjust() {
        let toml_str = r#"
[font]
family = "Menlo"

[font.adjust]
cell_width = 1.0
cell_height = 2.0
baseline = -1.5
"#;
        let config: crate::config::Config = toml::from_str(toml_str).unwrap();
        assert!((config.font.adjust.cell_width - 1.0).abs() < f32::EPSILON);
        assert!((config.font.adjust.cell_height - 2.0).abs() < f32::EPSILON);
        assert!((config.font.adjust.baseline - (-1.5)).abs() < f32::EPSILON);
    }

    #[test]
    fn deserialize_codepoint_map() {
        let toml_str = r#"
[font]
family = "Menlo"

[font.codepoint_map]
"U+3000-U+9FFF" = "Noto Sans CJK"
"U+1F000-U+1FFFF" = "Apple Color Emoji"
"#;
        let config: crate::config::Config = toml::from_str(toml_str).unwrap();
        let ranges = config.font.parsed_codepoint_map();
        assert_eq!(ranges.len(), 2);
    }

    #[test]
    fn font_config_empty_uses_defaults() {
        let config: crate::config::Config = toml::from_str("").unwrap();
        assert!(config.font.codepoint_map.is_empty());
        assert!(config.font.variation.is_empty());
        assert!(config.font.feature.is_empty());
        assert!((config.font.adjust.cell_width).abs() < f32::EPSILON);
    }

    // -----------------------------------------------------------------------
    // FontConfig::validate() テスト（M-1）
    // -----------------------------------------------------------------------

    #[test]
    fn validate_truncates_codepoint_map_over_64() {
        let mut config = FontConfig::default();
        for i in 0..70u32 {
            config.codepoint_map.insert(format!("U+{i:04X}"), format!("Font{i}"));
        }
        assert_eq!(config.codepoint_map.len(), 70);
        config.validate();
        assert_eq!(config.codepoint_map.len(), 64);
    }

    #[test]
    fn validate_truncates_variation_over_16() {
        let mut config = FontConfig::default();
        for i in 0..20u32 {
            config.variation.insert(format!("tag{i}"), i as f32);
        }
        assert_eq!(config.variation.len(), 20);
        config.validate();
        assert_eq!(config.variation.len(), 16);
    }

    #[test]
    fn validate_truncates_feature_over_32() {
        let mut config = FontConfig::default();
        for i in 0..40u32 {
            config.feature.insert(format!("tag{i}"), i % 2 == 0);
        }
        assert_eq!(config.feature.len(), 40);
        config.validate();
        assert_eq!(config.feature.len(), 32);
    }

    #[test]
    fn validate_keeps_entries_within_limits() {
        let mut config = FontConfig::default();
        config.codepoint_map.insert("U+3000-U+9FFF".to_owned(), "Noto CJK".to_owned());
        config.variation.insert("wght".to_owned(), 700.0);
        config.feature.insert("calt".to_owned(), true);
        config.validate();
        assert_eq!(config.codepoint_map.len(), 1);
        assert_eq!(config.variation.len(), 1);
        assert_eq!(config.feature.len(), 1);
    }

    // -----------------------------------------------------------------------
    // clamped_variation NaN/Inf フィルタテスト（M-2）
    // -----------------------------------------------------------------------

    #[test]
    fn clamped_variation_excludes_nan() {
        let mut config = FontConfig::default();
        config.variation.insert("wght".to_owned(), 700.0);
        config.variation.insert("wdth".to_owned(), f32::NAN);
        let result: Vec<_> = config.clamped_variation().collect();
        // wght のみ残る（NaN の wdth は除外される）
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "wght");
        assert!((result[0].1 - 700.0).abs() < f32::EPSILON);
    }

    #[test]
    fn clamped_variation_excludes_inf() {
        let mut config = FontConfig::default();
        config.variation.insert("wght".to_owned(), 700.0);
        config.variation.insert("ital".to_owned(), f32::INFINITY);
        config.variation.insert("slnt".to_owned(), f32::NEG_INFINITY);
        let result: Vec<_> = config.clamped_variation().collect();
        // wght のみ残る
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "wght");
    }

    #[test]
    fn clamped_variation_all_finite_passes_through() {
        let mut config = FontConfig::default();
        config.variation.insert("wght".to_owned(), 700.0);
        config.variation.insert("wdth".to_owned(), 100.0);
        let result: Vec<_> = config.clamped_variation().collect();
        assert_eq!(result.len(), 2);
    }
}
