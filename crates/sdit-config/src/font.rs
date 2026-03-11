use serde::Deserialize;

/// フォント設定。
#[derive(Debug, Clone, Deserialize)]
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
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            family: default_font_family().to_owned(),
            size: 14.0,
            line_height: 1.2,
            fallback_families: Vec::new(),
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
}
