use serde::{Deserialize, Serialize};

/// カラーテーマ設定。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct ColorConfig {
    /// 組み込みテーマ名。
    pub theme: ThemeName,
    /// 選択テキストの前景色（hex 文字列 "#RRGGBB"）。None = 背景色と反転した色を使用。
    pub selection_foreground: Option<String>,
    /// 選択テキストの背景色（hex 文字列 "#RRGGBB"）。None = 前景色と反転した色を使用。
    pub selection_background: Option<String>,
}

impl Default for ColorConfig {
    fn default() -> Self {
        Self {
            theme: ThemeName::CatppuccinMocha,
            selection_foreground: None,
            selection_background: None,
        }
    }
}

/// hex 文字列 "#RRGGBB" を `[f32; 4]` RGBA に変換する。
/// パース失敗時は `None` を返す。
pub fn parse_selection_color(hex: &str) -> Option<[f32; 4]> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some([f32::from(r) / 255.0, f32::from(g) / 255.0, f32::from(b) / 255.0, 1.0])
}

/// 組み込みテーマ名。
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum ThemeName {
    #[serde(rename = "catppuccin-mocha")]
    CatppuccinMocha,
    #[serde(rename = "catppuccin-latte")]
    CatppuccinLatte,
    #[serde(rename = "gruvbox-dark")]
    GruvboxDark,
}

/// 解決済みカラーテーブル（f32 RGBA）。レンダラーが直接使用する。
#[derive(Debug, Clone)]
pub struct ResolvedColors {
    /// ターミナル背景色。
    pub background: [f32; 4],
    /// ターミナル前景色。
    pub foreground: [f32; 4],
    /// サイドバー背景色。
    pub sidebar_bg: [f32; 4],
    /// サイドバーアクティブ行の背景色。
    pub sidebar_active_bg: [f32; 4],
    /// サイドバー前景色。
    pub sidebar_fg: [f32; 4],
    /// サイドバー非アクティブ行の前景色（dim）。
    pub sidebar_dim_fg: [f32; 4],
}

impl ResolvedColors {
    /// テーマ名から解決済みカラーを生成する。
    pub fn from_theme(theme: &ThemeName) -> Self {
        match theme {
            ThemeName::CatppuccinMocha => Self::catppuccin_mocha(),
            ThemeName::CatppuccinLatte => Self::catppuccin_latte(),
            ThemeName::GruvboxDark => Self::gruvbox_dark(),
        }
    }

    fn catppuccin_mocha() -> Self {
        Self {
            background: hex_to_rgba(0x1e, 0x1e, 0x2e),        // Base
            foreground: hex_to_rgba(0xcd, 0xd6, 0xf4),        // Text
            sidebar_bg: hex_to_rgba(0x31, 0x32, 0x44),        // Surface0
            sidebar_active_bg: hex_to_rgba(0x45, 0x47, 0x5a), // Surface1
            sidebar_fg: hex_to_rgba(0xcd, 0xd6, 0xf4),        // Text
            sidebar_dim_fg: hex_to_rgba(0x7f, 0x84, 0x9c),    // Overlay1（AA準拠に調整）
        }
    }

    fn catppuccin_latte() -> Self {
        Self {
            background: hex_to_rgba(0xef, 0xf1, 0xf5),        // Base
            foreground: hex_to_rgba(0x4c, 0x4f, 0x69),        // Text
            sidebar_bg: hex_to_rgba(0xcc, 0xd0, 0xda),        // Surface0
            sidebar_active_bg: hex_to_rgba(0xbc, 0xc0, 0xcc), // Surface1
            sidebar_fg: hex_to_rgba(0x4c, 0x4f, 0x69),        // Text
            sidebar_dim_fg: hex_to_rgba(0x6c, 0x6f, 0x85),    // Overlay0
        }
    }

    fn gruvbox_dark() -> Self {
        Self {
            background: hex_to_rgba(0x28, 0x28, 0x28),        // bg
            foreground: hex_to_rgba(0xeb, 0xdb, 0xb2),        // fg
            sidebar_bg: hex_to_rgba(0x3c, 0x38, 0x36),        // bg1
            sidebar_active_bg: hex_to_rgba(0x50, 0x49, 0x45), // bg2
            sidebar_fg: hex_to_rgba(0xeb, 0xdb, 0xb2),        // fg
            sidebar_dim_fg: hex_to_rgba(0xa8, 0x99, 0x84),    // fg4
        }
    }
}

impl Default for ResolvedColors {
    fn default() -> Self {
        Self::catppuccin_mocha()
    }
}

/// RGB バイト値を f32 RGBA に変換する（アルファは 1.0）。
fn hex_to_rgba(r: u8, g: u8, b: u8) -> [f32; 4] {
    [f32::from(r) / 255.0, f32::from(g) / 255.0, f32::from(b) / 255.0, 1.0]
}

/// WCAG 2.1 コントラスト比を計算する。
pub fn wcag_contrast_ratio(fg: [f32; 3], bg: [f32; 3]) -> f32 {
    let l1 = relative_luminance(fg);
    let l2 = relative_luminance(bg);
    let (lighter, darker) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
    (lighter + 0.05) / (darker + 0.05)
}

/// sRGB → 相対輝度（ITU-R BT.709）。
fn relative_luminance(rgb: [f32; 3]) -> f32 {
    let linearize =
        |c: f32| -> f32 { if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) } };
    0.2126 * linearize(rgb[0]) + 0.7152 * linearize(rgb[1]) + 0.0722 * linearize(rgb[2])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::float_cmp)]
    fn default_is_catppuccin_mocha() {
        let colors = ResolvedColors::default();
        let mocha = ResolvedColors::catppuccin_mocha();
        assert_eq!(colors.background, mocha.background);
    }

    #[test]
    fn from_theme_all_variants() {
        let _ = ResolvedColors::from_theme(&ThemeName::CatppuccinMocha);
        let _ = ResolvedColors::from_theme(&ThemeName::CatppuccinLatte);
        let _ = ResolvedColors::from_theme(&ThemeName::GruvboxDark);
    }

    #[test]
    fn contrast_black_white() {
        let ratio = wcag_contrast_ratio([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
        assert!(ratio > 20.0, "black/white ratio = {ratio:.1}");
    }

    #[test]
    fn contrast_same_color_is_one() {
        let ratio = wcag_contrast_ratio([0.5, 0.5, 0.5], [0.5, 0.5, 0.5]);
        assert!((ratio - 1.0).abs() < 0.01);
    }

    #[test]
    fn mocha_sidebar_fg_meets_wcag_aa() {
        let colors = ResolvedColors::catppuccin_mocha();
        let fg = [colors.sidebar_fg[0], colors.sidebar_fg[1], colors.sidebar_fg[2]];
        let bg =
            [colors.sidebar_active_bg[0], colors.sidebar_active_bg[1], colors.sidebar_active_bg[2]];
        let ratio = wcag_contrast_ratio(fg, bg);
        assert!(ratio >= 4.5, "sidebar fg/active_bg contrast = {ratio:.2}, expected >= 4.5");
    }

    #[test]
    fn mocha_sidebar_dim_meets_minimum() {
        let colors = ResolvedColors::catppuccin_mocha();
        let fg = [colors.sidebar_dim_fg[0], colors.sidebar_dim_fg[1], colors.sidebar_dim_fg[2]];
        let bg = [colors.sidebar_bg[0], colors.sidebar_bg[1], colors.sidebar_bg[2]];
        let ratio = wcag_contrast_ratio(fg, bg);
        assert!(ratio >= 3.0, "sidebar dim contrast = {ratio:.2}, expected >= 3.0");
    }

    #[test]
    fn deserialize_theme_name() {
        #[derive(Deserialize)]
        struct Test {
            theme: ThemeName,
        }
        let t: Test = toml::from_str(r#"theme = "catppuccin-latte""#).unwrap();
        assert_eq!(t.theme, ThemeName::CatppuccinLatte);
    }

    #[test]
    fn selection_color_config_default() {
        let cc = ColorConfig::default();
        assert!(cc.selection_foreground.is_none());
        assert!(cc.selection_background.is_none());
    }

    #[test]
    fn selection_color_config_deserialize() {
        let toml_str =
            "[colors]\nselection_foreground = \"#ffffff\"\nselection_background = \"#005577\"\n";
        #[derive(serde::Deserialize)]
        struct TestConfig {
            colors: ColorConfig,
        }
        let cfg: TestConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.colors.selection_foreground.as_deref(), Some("#ffffff"));
        assert_eq!(cfg.colors.selection_background.as_deref(), Some("#005577"));
    }

    #[test]
    fn parse_selection_color_valid() {
        let c = parse_selection_color("#ff0080").unwrap();
        assert!((c[0] - 1.0).abs() < 0.01);
        assert!((c[1] - 0.0).abs() < 0.01);
        assert!((c[3] - 1.0).abs() < 0.01);
    }

    #[test]
    fn parse_selection_color_invalid() {
        assert!(parse_selection_color("invalid").is_none());
        assert!(parse_selection_color("#gg0000").is_none());
        assert!(parse_selection_color("#fff").is_none());
    }

    /// 全テーマで fg/bg コントラスト比が WCAG AA (4.5:1) 以上であることを検証。
    #[test]
    fn all_themes_fg_bg_contrast_aa() {
        for theme in
            &[ThemeName::CatppuccinMocha, ThemeName::CatppuccinLatte, ThemeName::GruvboxDark]
        {
            let c = ResolvedColors::from_theme(theme);
            let fg = [c.foreground[0], c.foreground[1], c.foreground[2]];
            let bg = [c.background[0], c.background[1], c.background[2]];
            let ratio = wcag_contrast_ratio(fg, bg);
            assert!(ratio >= 4.5, "{theme:?}: fg/bg contrast = {ratio:.2}, expected >= 4.5");
        }
    }

    /// 全テーマでサイドバー fg/bg コントラスト比が AA 以上であることを検証。
    #[test]
    fn all_themes_sidebar_contrast_aa() {
        for theme in
            &[ThemeName::CatppuccinMocha, ThemeName::CatppuccinLatte, ThemeName::GruvboxDark]
        {
            let c = ResolvedColors::from_theme(theme);
            let fg = [c.sidebar_fg[0], c.sidebar_fg[1], c.sidebar_fg[2]];
            let bg = [c.sidebar_bg[0], c.sidebar_bg[1], c.sidebar_bg[2]];
            let ratio = wcag_contrast_ratio(fg, bg);
            assert!(
                ratio >= 4.5,
                "{theme:?}: sidebar fg/bg contrast = {ratio:.2}, expected >= 4.5"
            );
        }
    }

    /// 全テーマでサイドバー dim コントラスト比が 3:1 以上であることを検証。
    #[test]
    fn all_themes_sidebar_dim_contrast() {
        for theme in
            &[ThemeName::CatppuccinMocha, ThemeName::CatppuccinLatte, ThemeName::GruvboxDark]
        {
            let c = ResolvedColors::from_theme(theme);
            let fg = [c.sidebar_dim_fg[0], c.sidebar_dim_fg[1], c.sidebar_dim_fg[2]];
            let bg = [c.sidebar_bg[0], c.sidebar_bg[1], c.sidebar_bg[2]];
            let ratio = wcag_contrast_ratio(fg, bg);
            assert!(ratio >= 3.0, "{theme:?}: dim contrast = {ratio:.2}, expected >= 3.0");
        }
    }
}
