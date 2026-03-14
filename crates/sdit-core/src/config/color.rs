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
    /// セル描画時に保証する最小コントラスト比（WCAG 2.0 基準）。
    /// 1.0 = 無効（デフォルト）、最大 21.0。
    /// コントラスト比が不足する場合は fg の明度を自動調整する。
    pub minimum_contrast: f32,
}

impl Default for ColorConfig {
    fn default() -> Self {
        Self {
            theme: ThemeName::CatppuccinMocha,
            selection_foreground: None,
            selection_background: None,
            minimum_contrast: 1.0,
        }
    }
}

impl ColorConfig {
    /// minimum_contrast を安全な範囲（1.0〜21.0）にクランプして返す。
    ///
    /// NaN や Inf が渡された場合はデフォルト値 1.0 を返す（無効扱い）。
    pub fn clamped_minimum_contrast(&self) -> f32 {
        if self.minimum_contrast.is_finite() { self.minimum_contrast.clamp(1.0, 21.0) } else { 1.0 }
    }
}

/// hex 文字列 "#RRGGBB" を `[f32; 4]` RGBA に変換する。
/// パース失敗時は `None` を返す。
pub fn parse_selection_color(hex: &str) -> Option<[f32; 4]> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 || !hex.is_ascii() {
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
    #[serde(rename = "solarized-dark")]
    SolarizedDark,
    #[serde(rename = "solarized-light")]
    SolarizedLight,
    #[serde(rename = "dracula")]
    Dracula,
    #[serde(rename = "nord")]
    Nord,
    #[serde(rename = "one-dark")]
    OneDark,
    #[serde(rename = "tokyo-night")]
    TokyoNight,
}

impl ThemeName {
    /// 全テーマ名を定義順に返す。テーマサイクル操作に使用する。
    pub fn all() -> &'static [ThemeName] {
        &[
            ThemeName::CatppuccinMocha,
            ThemeName::CatppuccinLatte,
            ThemeName::GruvboxDark,
            ThemeName::SolarizedDark,
            ThemeName::SolarizedLight,
            ThemeName::Dracula,
            ThemeName::Nord,
            ThemeName::OneDark,
            ThemeName::TokyoNight,
        ]
    }

    /// 次のテーマを返す（末尾は先頭に折り返す）。
    pub fn next(&self) -> &'static ThemeName {
        let all = Self::all();
        let idx = all.iter().position(|t| t == self).unwrap_or(0);
        &all[(idx + 1) % all.len()]
    }

    /// 前のテーマを返す（先頭は末尾に折り返す）。
    pub fn prev(&self) -> &'static ThemeName {
        let all = Self::all();
        let idx = all.iter().position(|t| t == self).unwrap_or(0);
        &all[(idx + all.len() - 1) % all.len()]
    }
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
            ThemeName::SolarizedDark => Self::solarized_dark(),
            ThemeName::SolarizedLight => Self::solarized_light(),
            ThemeName::Dracula => Self::dracula(),
            ThemeName::Nord => Self::nord(),
            ThemeName::OneDark => Self::one_dark(),
            ThemeName::TokyoNight => Self::tokyo_night(),
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

    fn solarized_dark() -> Self {
        Self {
            background: hex_to_rgba(0x00, 0x2b, 0x36),        // base03
            foreground: hex_to_rgba(0x83, 0x94, 0x96),        // base0（contrast 4.11 on base03）
            sidebar_bg: hex_to_rgba(0x07, 0x36, 0x42),        // base02
            sidebar_active_bg: hex_to_rgba(0x58, 0x6e, 0x75), // base01
            sidebar_fg: hex_to_rgba(0x93, 0xa1, 0xa1),        // base1（contrast 4.86 on base02）
            sidebar_dim_fg: hex_to_rgba(0x70, 0x7e, 0x80),    // adjusted（contrast 3.09 on base02）
        }
    }

    fn solarized_light() -> Self {
        Self {
            background: hex_to_rgba(0xfd, 0xf6, 0xe3),        // base3
            foreground: hex_to_rgba(0x58, 0x6e, 0x75),        // base01（contrast 4.99 on base3）
            sidebar_bg: hex_to_rgba(0xee, 0xe8, 0xd5),        // base2
            sidebar_active_bg: hex_to_rgba(0x93, 0xa1, 0xa1), // base1
            sidebar_fg: hex_to_rgba(0x48, 0x6e, 0x72),        // adjusted（contrast 4.57 on base2）
            sidebar_dim_fg: hex_to_rgba(0x65, 0x7b, 0x83),    // base00（contrast 3.64 on base2）
        }
    }

    fn dracula() -> Self {
        Self {
            background: hex_to_rgba(0x28, 0x2a, 0x36), // Background
            foreground: hex_to_rgba(0xf8, 0xf8, 0xf2), // Foreground
            sidebar_bg: hex_to_rgba(0x44, 0x47, 0x5a), // Current Line
            sidebar_active_bg: hex_to_rgba(0x6e, 0x72, 0x82), // Comment（少し明るめ）
            sidebar_fg: hex_to_rgba(0xf8, 0xf8, 0xf2), // Foreground
            sidebar_dim_fg: hex_to_rgba(0xbd, 0xc3, 0xd0), // dimmed foreground（AA準拠）
        }
    }

    fn nord() -> Self {
        Self {
            background: hex_to_rgba(0x2e, 0x34, 0x40),        // nord0
            foreground: hex_to_rgba(0xd8, 0xde, 0xe9),        // nord4
            sidebar_bg: hex_to_rgba(0x3b, 0x42, 0x52),        // nord1
            sidebar_active_bg: hex_to_rgba(0x43, 0x4c, 0x5e), // nord2
            sidebar_fg: hex_to_rgba(0xd8, 0xde, 0xe9),        // nord4
            sidebar_dim_fg: hex_to_rgba(0xa0, 0xa8, 0xb8),    // nord3 adjusted for AA
        }
    }

    fn one_dark() -> Self {
        Self {
            background: hex_to_rgba(0x28, 0x2c, 0x34), // Atom One Dark bg
            foreground: hex_to_rgba(0xab, 0xb2, 0xbf), // fg
            sidebar_bg: hex_to_rgba(0x21, 0x25, 0x2b), // slightly darker
            sidebar_active_bg: hex_to_rgba(0x3e, 0x44, 0x51), // selection bg
            sidebar_fg: hex_to_rgba(0xab, 0xb2, 0xbf), // fg
            sidebar_dim_fg: hex_to_rgba(0x7f, 0x84, 0x8e), // comment
        }
    }

    fn tokyo_night() -> Self {
        Self {
            background: hex_to_rgba(0x1a, 0x1b, 0x26),        // bg
            foreground: hex_to_rgba(0xc0, 0xca, 0xf5),        // fg
            sidebar_bg: hex_to_rgba(0x16, 0x17, 0x20),        // bg_dark
            sidebar_active_bg: hex_to_rgba(0x29, 0x2e, 0x42), // bg_highlight
            sidebar_fg: hex_to_rgba(0xc0, 0xca, 0xf5),        // fg
            sidebar_dim_fg: hex_to_rgba(0x70, 0x82, 0xa4), // adjusted（contrast 4.60 on bg_dark）
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
pub fn relative_luminance(rgb: [f32; 3]) -> f32 {
    let linearize =
        |c: f32| -> f32 { if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) } };
    0.2126 * linearize(rgb[0]) + 0.7152 * linearize(rgb[1]) + 0.0722 * linearize(rgb[2])
}

/// 最小コントラスト比を満たすように fg 色を調整して返す。
///
/// - `minimum_contrast` が 1.0 以下の場合は fg をそのまま返す（パフォーマンス考慮）。
/// - bg が暗い場合（輝度 < 0.5）は fg を明るく調整する。
/// - bg が明るい場合（輝度 >= 0.5）は fg を暗く調整する。
/// - 100 ステップで収束しなかった場合は最後に計算した値を返す。
pub fn apply_minimum_contrast(fg: [f32; 3], bg: [f32; 3], minimum_contrast: f32) -> [f32; 3] {
    // 1.0 以下は無効（デフォルト）
    if minimum_contrast <= 1.0 {
        return fg;
    }

    let current_ratio = wcag_contrast_ratio(fg, bg);
    if current_ratio >= minimum_contrast {
        return fg;
    }

    let bg_lum = relative_luminance(bg);
    let bg_is_dark = bg_lum < 0.5;

    // HSV の V（明度）を調整して目標コントラスト比に近づける
    // fg を RGB → [0.0, 1.0] のスカラー明度でスケールする簡易実装
    let mut adjusted = fg;
    let step = if bg_is_dark { 0.01_f32 } else { -0.01_f32 };

    for _ in 0..200 {
        let ratio = wcag_contrast_ratio(adjusted, bg);
        if ratio >= minimum_contrast {
            break;
        }
        adjusted[0] = (adjusted[0] + step).clamp(0.0, 1.0);
        adjusted[1] = (adjusted[1] + step).clamp(0.0, 1.0);
        adjusted[2] = (adjusted[2] + step).clamp(0.0, 1.0);
    }

    adjusted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimum_contrast_default_is_one() {
        let cc = ColorConfig::default();
        assert!((cc.minimum_contrast - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn clamped_minimum_contrast_clamps_range() {
        let mut cc = ColorConfig::default();
        cc.minimum_contrast = 0.5;
        assert!((cc.clamped_minimum_contrast() - 1.0).abs() < f32::EPSILON);
        cc.minimum_contrast = 25.0;
        assert!((cc.clamped_minimum_contrast() - 21.0).abs() < f32::EPSILON);
        cc.minimum_contrast = 4.5;
        assert!((cc.clamped_minimum_contrast() - 4.5).abs() < f32::EPSILON);
    }

    #[test]
    fn clamped_minimum_contrast_rejects_nan_inf() {
        // NaN や無限大は非有限数として 1.0（無効）を返す
        let mut cc = ColorConfig::default();
        cc.minimum_contrast = f32::NAN;
        assert!(
            (cc.clamped_minimum_contrast() - 1.0).abs() < f32::EPSILON,
            "NaN は 1.0 を返すべき"
        );
        cc.minimum_contrast = f32::INFINITY;
        // INFINITY は is_finite() が false → デフォルト 1.0 を返す（安全側）
        assert!(
            (cc.clamped_minimum_contrast() - 1.0).abs() < f32::EPSILON,
            "INFINITY は 1.0 を返すべき（安全側）"
        );
        cc.minimum_contrast = f32::NEG_INFINITY;
        assert!(
            (cc.clamped_minimum_contrast() - 1.0).abs() < f32::EPSILON,
            "NEG_INFINITY は 1.0 を返すべき"
        );
    }

    #[test]
    fn minimum_contrast_config_deserialize() {
        let toml_str = "[colors]\nminimum_contrast = 4.5\n";
        #[derive(serde::Deserialize)]
        struct TestConfig {
            colors: ColorConfig,
        }
        let cfg: TestConfig = toml::from_str(toml_str).unwrap();
        assert!((cfg.colors.minimum_contrast - 4.5).abs() < f32::EPSILON);
    }

    #[test]
    fn apply_minimum_contrast_no_adjustment_when_disabled() {
        let fg = [0.5_f32, 0.5, 0.5];
        let bg = [0.1_f32, 0.1, 0.1];
        let result = apply_minimum_contrast(fg, bg, 1.0);
        assert_eq!(result, fg, "minimum_contrast=1.0 は何も変更しない");
    }

    #[test]
    fn apply_minimum_contrast_brightens_fg_on_dark_bg() {
        // 暗い背景に暗いグレーの前景 → 明るく調整される
        let fg = [0.3_f32, 0.3, 0.3];
        let bg = [0.05_f32, 0.05, 0.05];
        let adjusted = apply_minimum_contrast(fg, bg, 4.5);
        let ratio = wcag_contrast_ratio(adjusted, bg);
        assert!(ratio >= 4.5, "調整後のコントラスト比 {ratio:.2} が目標 4.5 未満");
    }

    #[test]
    fn apply_minimum_contrast_darkens_fg_on_light_bg() {
        // 明るい背景に明るいグレーの前景 → 暗く調整される
        let fg = [0.8_f32, 0.8, 0.8];
        let bg = [0.95_f32, 0.95, 0.95];
        let adjusted = apply_minimum_contrast(fg, bg, 4.5);
        let ratio = wcag_contrast_ratio(adjusted, bg);
        assert!(ratio >= 4.5, "調整後のコントラスト比 {ratio:.2} が目標 4.5 未満");
    }

    #[test]
    fn apply_minimum_contrast_no_change_when_already_sufficient() {
        // 白背景に黒テキスト → コントラスト比は最大（調整不要）
        let fg = [0.0_f32, 0.0, 0.0];
        let bg = [1.0_f32, 1.0, 1.0];
        let result = apply_minimum_contrast(fg, bg, 4.5);
        assert_eq!(result, fg, "十分なコントラスト比がある場合は変更しない");
    }

    #[test]
    fn relative_luminance_black_zero() {
        let lum = relative_luminance([0.0, 0.0, 0.0]);
        assert!(lum.abs() < f32::EPSILON, "黒の輝度は 0.0");
    }

    #[test]
    fn relative_luminance_white_one() {
        let lum = relative_luminance([1.0, 1.0, 1.0]);
        assert!((lum - 1.0).abs() < 0.001, "白の輝度は 1.0");
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn default_is_catppuccin_mocha() {
        let colors = ResolvedColors::default();
        let mocha = ResolvedColors::catppuccin_mocha();
        assert_eq!(colors.background, mocha.background);
    }

    #[test]
    fn from_theme_all_variants() {
        for theme in ThemeName::all() {
            let _ = ResolvedColors::from_theme(theme);
        }
    }

    #[test]
    fn theme_next_cycles() {
        let all = ThemeName::all();
        // 最後のテーマの次は先頭に戻る
        let last = all.last().unwrap();
        let next = last.next();
        assert_eq!(next, &all[0]);
        // 先頭テーマの前は末尾に戻る
        let first = &all[0];
        let prev = first.prev();
        assert_eq!(prev, all.last().unwrap());
    }

    #[test]
    fn theme_name_serde_roundtrip() {
        let names = [
            ("solarized-dark", ThemeName::SolarizedDark),
            ("solarized-light", ThemeName::SolarizedLight),
            ("dracula", ThemeName::Dracula),
            ("nord", ThemeName::Nord),
            ("one-dark", ThemeName::OneDark),
            ("tokyo-night", ThemeName::TokyoNight),
        ];
        for (s, expected) in &names {
            #[derive(serde::Deserialize)]
            struct T {
                theme: ThemeName,
            }
            let t: T = toml::from_str(&format!(r#"theme = "{s}""#)).unwrap();
            assert_eq!(&t.theme, expected, "failed for {s}");
        }
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

    #[test]
    fn parse_selection_color_non_ascii_rejected() {
        // 非 ASCII 文字を含む入力はバイトインデックスアクセスが安全でないため拒否する
        assert!(parse_selection_color("#ff00\u{00e9}\u{00e9}").is_none());
        assert!(parse_selection_color("ａｂｃｄｅｆ").is_none());
    }

    /// 全テーマで fg/bg コントラスト比が WCAG AA (4.5:1) 以上であることを検証。
    #[test]
    fn all_themes_fg_bg_contrast_aa() {
        for theme in ThemeName::all() {
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
        for theme in ThemeName::all() {
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
        for theme in ThemeName::all() {
            let c = ResolvedColors::from_theme(theme);
            let fg = [c.sidebar_dim_fg[0], c.sidebar_dim_fg[1], c.sidebar_dim_fg[2]];
            let bg = [c.sidebar_bg[0], c.sidebar_bg[1], c.sidebar_bg[2]];
            let ratio = wcag_contrast_ratio(fg, bg);
            assert!(ratio >= 3.0, "{theme:?}: dim contrast = {ratio:.2}, expected >= 3.0");
        }
    }
}
