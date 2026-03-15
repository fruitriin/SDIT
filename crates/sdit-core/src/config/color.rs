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
    /// 検索ハイライトの前景色（hex 文字列 "#RRGGBB"）。None = デフォルト色を使用。
    pub search_foreground: Option<String>,
    /// 検索ハイライトの背景色（hex 文字列 "#RRGGBB"）。None = デフォルト色を使用。
    pub search_background: Option<String>,
    /// 太字テキストを明色（Bright variant）に変換する（デフォルト: false）。
    ///
    /// `true` の場合、`CellFlags::BOLD` が立っているセルの fg が通常 Named 色
    /// （Black..White）であれば対応する BrightBlack..BrightWhite に変換して描画する。
    pub bold_is_bright: bool,
    /// SGR 2（DIM/FAINT）のアルファ倍率（デフォルト: 0.5）。
    ///
    /// `CellFlags::DIM` が立っているセルの fg アルファをこの値で乗算する。
    /// 有効範囲: 0.0〜1.0。NaN は 0.5 として扱う。
    pub faint_opacity: f32,
    /// bg/fg から ANSI 16色パレットを自動生成する（デフォルト: false）。
    ///
    /// `true` の場合、HSL 補間により背景色・前景色からパレットを生成する。
    /// テーマ固有のパレットより優先される。
    pub palette_generate: bool,
    /// `palette_generate = true` のときに暗・明テーマを自動適応する（デフォルト: false）。
    ///
    /// `true` の場合、生成パレットの normal と bright の lightness を入れ替えて
    /// ライトテーマ向けに適応する。`palette_generate` と組み合わせて使用する。
    pub palette_harmonious: bool,
}

impl Default for ColorConfig {
    fn default() -> Self {
        Self {
            theme: ThemeName::CatppuccinMocha,
            selection_foreground: None,
            selection_background: None,
            minimum_contrast: 1.0,
            search_foreground: None,
            search_background: None,
            bold_is_bright: false,
            faint_opacity: 0.5,
            palette_generate: false,
            palette_harmonious: false,
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

    /// faint_opacity を安全な範囲（0.0〜1.0）にクランプして返す。
    ///
    /// NaN や非有限値が渡された場合はデフォルト値 0.5 を返す。
    pub fn clamped_faint_opacity(&self) -> f32 {
        if self.faint_opacity.is_finite() { self.faint_opacity.clamp(0.0, 1.0) } else { 0.5 }
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
    /// ANSI 16色パレット。インデックス: [0]=Black, [1]=Red, ..., [7]=White,
    /// [8]=BrightBlack, ..., [15]=BrightWhite。
    pub ansi_palette: [[f32; 4]; 16],
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

    /// `ColorConfig` から解決済みカラーを生成する。
    ///
    /// `palette_generate = true` の場合、bg/fg から ANSI 16色パレットを HSL 補間で生成する。
    /// `palette_harmonious = true` の場合、normal と bright の lightness を入れ替える。
    pub fn from_color_config(config: &ColorConfig) -> Self {
        let mut resolved = Self::from_theme(&config.theme);
        if config.palette_generate {
            resolved.ansi_palette = generate_ansi_palette(resolved.background, resolved.foreground);
            if config.palette_harmonious {
                apply_harmonious(&mut resolved.ansi_palette);
            }
        }
        resolved
    }

    fn catppuccin_mocha() -> Self {
        Self {
            background: hex_to_rgba(0x1e, 0x1e, 0x2e),        // Base
            foreground: hex_to_rgba(0xcd, 0xd6, 0xf4),        // Text
            sidebar_bg: hex_to_rgba(0x31, 0x32, 0x44),        // Surface0
            sidebar_active_bg: hex_to_rgba(0x45, 0x47, 0x5a), // Surface1
            sidebar_fg: hex_to_rgba(0xcd, 0xd6, 0xf4),        // Text
            sidebar_dim_fg: hex_to_rgba(0x7f, 0x84, 0x9c),    // Overlay1（AA準拠に調整）
            ansi_palette: [
                hex_to_rgba(0x45, 0x47, 0x5a), // 0=Black (Surface0)
                hex_to_rgba(0xf3, 0x8b, 0xa8), // 1=Red
                hex_to_rgba(0xa6, 0xe3, 0xa1), // 2=Green
                hex_to_rgba(0xf9, 0xe2, 0xaf), // 3=Yellow
                hex_to_rgba(0x89, 0xb4, 0xfa), // 4=Blue
                hex_to_rgba(0xcb, 0xa6, 0xf7), // 5=Magenta
                hex_to_rgba(0x89, 0xdc, 0xeb), // 6=Cyan
                hex_to_rgba(0xba, 0xc2, 0xde), // 7=White
                hex_to_rgba(0x58, 0x5b, 0x70), // 8=BrightBlack (Surface2)
                hex_to_rgba(0xf3, 0x8b, 0xa8), // 9=BrightRed
                hex_to_rgba(0xa6, 0xe3, 0xa1), // 10=BrightGreen
                hex_to_rgba(0xf9, 0xe2, 0xaf), // 11=BrightYellow
                hex_to_rgba(0x89, 0xb4, 0xfa), // 12=BrightBlue
                hex_to_rgba(0xcb, 0xa6, 0xf7), // 13=BrightMagenta
                hex_to_rgba(0x89, 0xdc, 0xeb), // 14=BrightCyan
                hex_to_rgba(0xa6, 0xad, 0xc8), // 15=BrightWhite (Subtext0)
            ],
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
            ansi_palette: [
                hex_to_rgba(0x5c, 0x5f, 0x77), // 0=Black
                hex_to_rgba(0xd2, 0x0f, 0x39), // 1=Red
                hex_to_rgba(0x40, 0xa0, 0x2b), // 2=Green
                hex_to_rgba(0xdf, 0x8e, 0x1d), // 3=Yellow
                hex_to_rgba(0x1e, 0x66, 0xf5), // 4=Blue
                hex_to_rgba(0x88, 0x39, 0xef), // 5=Magenta
                hex_to_rgba(0x04, 0xa5, 0xe5), // 6=Cyan
                hex_to_rgba(0xac, 0xb0, 0xbe), // 7=White
                hex_to_rgba(0x6c, 0x6f, 0x85), // 8=BrightBlack
                hex_to_rgba(0xd2, 0x0f, 0x39), // 9=BrightRed
                hex_to_rgba(0x40, 0xa0, 0x2b), // 10=BrightGreen
                hex_to_rgba(0xdf, 0x8e, 0x1d), // 11=BrightYellow
                hex_to_rgba(0x1e, 0x66, 0xf5), // 12=BrightBlue
                hex_to_rgba(0x88, 0x39, 0xef), // 13=BrightMagenta
                hex_to_rgba(0x04, 0xa5, 0xe5), // 14=BrightCyan
                hex_to_rgba(0xbc, 0xc0, 0xcc), // 15=BrightWhite
            ],
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
            ansi_palette: [
                hex_to_rgba(0x28, 0x28, 0x28), // 0=Black
                hex_to_rgba(0xcc, 0x24, 0x1d), // 1=Red
                hex_to_rgba(0x98, 0x97, 0x1a), // 2=Green
                hex_to_rgba(0xd7, 0x99, 0x21), // 3=Yellow
                hex_to_rgba(0x45, 0x85, 0x88), // 4=Blue
                hex_to_rgba(0xb1, 0x62, 0x86), // 5=Magenta
                hex_to_rgba(0x68, 0x9d, 0x6a), // 6=Cyan
                hex_to_rgba(0xa8, 0x99, 0x84), // 7=White
                hex_to_rgba(0x92, 0x83, 0x74), // 8=BrightBlack
                hex_to_rgba(0xfb, 0x49, 0x34), // 9=BrightRed
                hex_to_rgba(0xb8, 0xbb, 0x26), // 10=BrightGreen
                hex_to_rgba(0xfa, 0xbd, 0x2f), // 11=BrightYellow
                hex_to_rgba(0x83, 0xa5, 0x98), // 12=BrightBlue
                hex_to_rgba(0xd3, 0x86, 0x9b), // 13=BrightMagenta
                hex_to_rgba(0x8e, 0xc0, 0x7c), // 14=BrightCyan
                hex_to_rgba(0xeb, 0xdb, 0xb2), // 15=BrightWhite
            ],
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
            ansi_palette: [
                hex_to_rgba(0x07, 0x36, 0x42), // 0=Black (base02)
                hex_to_rgba(0xdc, 0x32, 0x2f), // 1=Red
                hex_to_rgba(0x85, 0x99, 0x00), // 2=Green
                hex_to_rgba(0xb5, 0x89, 0x00), // 3=Yellow
                hex_to_rgba(0x26, 0x8b, 0xd2), // 4=Blue
                hex_to_rgba(0xd3, 0x36, 0x82), // 5=Magenta
                hex_to_rgba(0x2a, 0xa1, 0x98), // 6=Cyan
                hex_to_rgba(0xee, 0xe8, 0xd5), // 7=White (base2)
                hex_to_rgba(0x00, 0x2b, 0x36), // 8=BrightBlack (base03)
                hex_to_rgba(0xcb, 0x4b, 0x16), // 9=BrightRed (orange)
                hex_to_rgba(0x58, 0x6e, 0x75), // 10=BrightGreen (base01)
                hex_to_rgba(0x65, 0x7b, 0x83), // 11=BrightYellow (base00)
                hex_to_rgba(0x83, 0x94, 0x96), // 12=BrightBlue (base0)
                hex_to_rgba(0x6c, 0x71, 0xc4), // 13=BrightMagenta (violet)
                hex_to_rgba(0x93, 0xa1, 0xa1), // 14=BrightCyan (base1)
                hex_to_rgba(0xfd, 0xf6, 0xe3), // 15=BrightWhite (base3)
            ],
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
            ansi_palette: [
                hex_to_rgba(0xee, 0xe8, 0xd5), // 0=Black (base2)
                hex_to_rgba(0xdc, 0x32, 0x2f), // 1=Red
                hex_to_rgba(0x85, 0x99, 0x00), // 2=Green
                hex_to_rgba(0xb5, 0x89, 0x00), // 3=Yellow
                hex_to_rgba(0x26, 0x8b, 0xd2), // 4=Blue
                hex_to_rgba(0xd3, 0x36, 0x82), // 5=Magenta
                hex_to_rgba(0x2a, 0xa1, 0x98), // 6=Cyan
                hex_to_rgba(0x07, 0x36, 0x42), // 7=White (base02)
                hex_to_rgba(0xfd, 0xf6, 0xe3), // 8=BrightBlack (base3)
                hex_to_rgba(0xcb, 0x4b, 0x16), // 9=BrightRed (orange)
                hex_to_rgba(0x93, 0xa1, 0xa1), // 10=BrightGreen (base1)
                hex_to_rgba(0x83, 0x94, 0x96), // 11=BrightYellow (base0)
                hex_to_rgba(0x65, 0x7b, 0x83), // 12=BrightBlue (base00)
                hex_to_rgba(0x6c, 0x71, 0xc4), // 13=BrightMagenta (violet)
                hex_to_rgba(0x58, 0x6e, 0x75), // 14=BrightCyan (base01)
                hex_to_rgba(0x00, 0x2b, 0x36), // 15=BrightWhite (base03)
            ],
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
            ansi_palette: [
                hex_to_rgba(0x21, 0x22, 0x2c), // 0=Black
                hex_to_rgba(0xff, 0x55, 0x55), // 1=Red
                hex_to_rgba(0x50, 0xfa, 0x7b), // 2=Green
                hex_to_rgba(0xf1, 0xfa, 0x8c), // 3=Yellow
                hex_to_rgba(0xbd, 0x93, 0xf9), // 4=Blue
                hex_to_rgba(0xff, 0x79, 0xc6), // 5=Magenta
                hex_to_rgba(0x8b, 0xe9, 0xfd), // 6=Cyan
                hex_to_rgba(0xf8, 0xf8, 0xf2), // 7=White
                hex_to_rgba(0x62, 0x72, 0xa4), // 8=BrightBlack (comment)
                hex_to_rgba(0xff, 0x6e, 0x6e), // 9=BrightRed
                hex_to_rgba(0x69, 0xff, 0x94), // 10=BrightGreen
                hex_to_rgba(0xff, 0xff, 0xa5), // 11=BrightYellow
                hex_to_rgba(0xd6, 0xac, 0xff), // 12=BrightBlue
                hex_to_rgba(0xff, 0x92, 0xdf), // 13=BrightMagenta
                hex_to_rgba(0xa4, 0xff, 0xff), // 14=BrightCyan
                hex_to_rgba(0xff, 0xff, 0xff), // 15=BrightWhite
            ],
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
            ansi_palette: [
                hex_to_rgba(0x3b, 0x42, 0x52), // 0=Black (nord1)
                hex_to_rgba(0xbf, 0x61, 0x6a), // 1=Red (nord11)
                hex_to_rgba(0xa3, 0xbe, 0x8c), // 2=Green (nord14)
                hex_to_rgba(0xeb, 0xcb, 0x8b), // 3=Yellow (nord13)
                hex_to_rgba(0x81, 0xa1, 0xc1), // 4=Blue (nord9)
                hex_to_rgba(0xb4, 0x8e, 0xad), // 5=Magenta (nord15)
                hex_to_rgba(0x88, 0xc0, 0xd0), // 6=Cyan (nord8)
                hex_to_rgba(0xe5, 0xe9, 0xf0), // 7=White (nord5)
                hex_to_rgba(0x4c, 0x56, 0x6a), // 8=BrightBlack (nord3)
                hex_to_rgba(0xbf, 0x61, 0x6a), // 9=BrightRed
                hex_to_rgba(0xa3, 0xbe, 0x8c), // 10=BrightGreen
                hex_to_rgba(0xeb, 0xcb, 0x8b), // 11=BrightYellow
                hex_to_rgba(0x81, 0xa1, 0xc1), // 12=BrightBlue
                hex_to_rgba(0xb4, 0x8e, 0xad), // 13=BrightMagenta
                hex_to_rgba(0x8f, 0xbc, 0xbb), // 14=BrightCyan (nord7)
                hex_to_rgba(0xec, 0xef, 0xf4), // 15=BrightWhite (nord6)
            ],
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
            ansi_palette: [
                hex_to_rgba(0x28, 0x2c, 0x34), // 0=Black
                hex_to_rgba(0xe0, 0x6c, 0x75), // 1=Red
                hex_to_rgba(0x98, 0xc3, 0x79), // 2=Green
                hex_to_rgba(0xe5, 0xc0, 0x7b), // 3=Yellow
                hex_to_rgba(0x61, 0xaf, 0xef), // 4=Blue
                hex_to_rgba(0xc6, 0x78, 0xdd), // 5=Magenta
                hex_to_rgba(0x56, 0xb6, 0xc2), // 6=Cyan
                hex_to_rgba(0xab, 0xb2, 0xbf), // 7=White
                hex_to_rgba(0x5c, 0x63, 0x70), // 8=BrightBlack (comment)
                hex_to_rgba(0xe0, 0x6c, 0x75), // 9=BrightRed
                hex_to_rgba(0x98, 0xc3, 0x79), // 10=BrightGreen
                hex_to_rgba(0xe5, 0xc0, 0x7b), // 11=BrightYellow
                hex_to_rgba(0x61, 0xaf, 0xef), // 12=BrightBlue
                hex_to_rgba(0xc6, 0x78, 0xdd), // 13=BrightMagenta
                hex_to_rgba(0x56, 0xb6, 0xc2), // 14=BrightCyan
                hex_to_rgba(0xc8, 0xcd, 0xd7), // 15=BrightWhite
            ],
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
            ansi_palette: [
                hex_to_rgba(0x15, 0x16, 0x1e), // 0=Black
                hex_to_rgba(0xf7, 0x76, 0x8e), // 1=Red
                hex_to_rgba(0x9e, 0xce, 0x6a), // 2=Green
                hex_to_rgba(0xe0, 0xaf, 0x68), // 3=Yellow
                hex_to_rgba(0x7a, 0xa2, 0xf7), // 4=Blue
                hex_to_rgba(0xbb, 0x9a, 0xf7), // 5=Magenta
                hex_to_rgba(0x7d, 0xcf, 0xff), // 6=Cyan
                hex_to_rgba(0xa9, 0xb1, 0xd6), // 7=White
                hex_to_rgba(0x41, 0x48, 0x68), // 8=BrightBlack
                hex_to_rgba(0xf7, 0x76, 0x8e), // 9=BrightRed
                hex_to_rgba(0x9e, 0xce, 0x6a), // 10=BrightGreen
                hex_to_rgba(0xe0, 0xaf, 0x68), // 11=BrightYellow
                hex_to_rgba(0x7a, 0xa2, 0xf7), // 12=BrightBlue
                hex_to_rgba(0xbb, 0x9a, 0xf7), // 13=BrightMagenta
                hex_to_rgba(0x7d, 0xcf, 0xff), // 14=BrightCyan
                hex_to_rgba(0xc0, 0xca, 0xf5), // 15=BrightWhite
            ],
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

// ---------------------------------------------------------------------------
// パレット生成（HSL 補間）
// ---------------------------------------------------------------------------

/// RGB を HSL に変換する。
/// 返値: (h: 0.0..1.0, s: 0.0..1.0, l: 0.0..1.0)
fn rgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    if (max - min).abs() < f32::EPSILON {
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
    let h = if (max - r).abs() < f32::EPSILON {
        let seg = (g - b) / d;
        if g < b { seg + 6.0 } else { seg }
    } else if (max - g).abs() < f32::EPSILON {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    (h / 6.0, s, l)
}

/// HSL を RGB に変換する。
fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    if s.abs() < f32::EPSILON {
        return (l, l, l);
    }
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    let hue_to_rgb = |t: f32| -> f32 {
        let t = t.rem_euclid(1.0);
        if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        }
    };
    (hue_to_rgb(h + 1.0 / 3.0), hue_to_rgb(h), hue_to_rgb(h - 1.0 / 3.0))
}

/// bg/fg から ANSI 16色パレットを HSL 補間で生成する。
fn generate_ansi_palette(bg: [f32; 4], fg: [f32; 4]) -> [[f32; 4]; 16] {
    // apply_minimum_contrast と同じ相対輝度計算を使用（ITU-R BT.709 準拠）
    let bg_lum = relative_luminance([bg[0], bg[1], bg[2]]);
    let is_dark = bg_lum < 0.5;
    let sat = 0.75_f32;
    let lum = if is_dark { 0.60 } else { 0.40 };
    let bright_lum = if is_dark { 0.72 } else { 0.30 };

    let make = |h: f32, l: f32| -> [f32; 4] {
        let (r, g, b) = hsl_to_rgb(h, sat, l);
        [r, g, b, 1.0]
    };
    let lerp = |a: [f32; 4], b: [f32; 4], t: f32| -> [f32; 4] {
        [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t, 1.0]
    };

    let dark_factor = if is_dark { 0.15 } else { 0.20 };
    [
        lerp(bg, fg, dark_factor),                       // 0=Black
        make(0.0, lum),                                  // 1=Red
        make(1.0 / 3.0, lum),                            // 2=Green
        make(1.0 / 6.0, lum),                            // 3=Yellow
        make(2.0 / 3.0, lum),                            // 4=Blue
        make(5.0 / 6.0, lum),                            // 5=Magenta
        make(0.5, lum),                                  // 6=Cyan
        lerp(bg, fg, if is_dark { 0.75 } else { 0.85 }), // 7=White
        lerp(bg, fg, dark_factor * 2.0),                 // 8=BrightBlack
        make(0.0, bright_lum),                           // 9=BrightRed
        make(1.0 / 3.0, bright_lum),                     // 10=BrightGreen
        make(1.0 / 6.0, bright_lum),                     // 11=BrightYellow
        make(2.0 / 3.0, bright_lum),                     // 12=BrightBlue
        make(5.0 / 6.0, bright_lum),                     // 13=BrightMagenta
        make(0.5, bright_lum),                           // 14=BrightCyan
        fg,                                              // 15=BrightWhite
    ]
}

/// palette_harmonious: normal と bright の lightness を入れ替えてライトテーマ適応する。
fn apply_harmonious(palette: &mut [[f32; 4]; 16]) {
    // インデックス 1-6 (normal) と 9-14 (bright) の間で lightness を入れ替える
    for i in 1..=6 {
        let n = palette[i];
        let b = palette[i + 8];
        let (hn, sn, ln) = rgb_to_hsl(n[0], n[1], n[2]);
        let (hb, sb, lb) = rgb_to_hsl(b[0], b[1], b[2]);
        let (rn, gn, bn) = hsl_to_rgb(hn, sn, lb); // normal に bright の lightness を使用
        let (rb, gb, bb) = hsl_to_rgb(hb, sb, ln); // bright に normal の lightness を使用
        palette[i] = [rn, gn, bn, 1.0];
        palette[i + 8] = [rb, gb, bb, 1.0];
    }
    // 7=White と 15=BrightWhite も入れ替え
    let w = palette[7];
    let bw = palette[15];
    let (hw, sw, lw) = rgb_to_hsl(w[0], w[1], w[2]);
    let (hbw, sbw, lbw) = rgb_to_hsl(bw[0], bw[1], bw[2]);
    let (rw, gw, bw_rgb) = hsl_to_rgb(hw, sw, lbw);
    let (rbw, gbw, bbw) = hsl_to_rgb(hbw, sbw, lw);
    palette[7] = [rw, gw, bw_rgb, 1.0];
    palette[15] = [rbw, gbw, bbw, 1.0];
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
            let colors = ResolvedColors::from_theme(theme);
            assert_ne!(colors.background, colors.foreground, "theme {theme:?}: bg == fg");
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
    fn bold_is_bright_config_default() {
        let cc = ColorConfig::default();
        assert!(!cc.bold_is_bright, "bold_is_bright のデフォルトは false");
    }

    #[test]
    fn faint_opacity_config_clamp() {
        let mut cc = ColorConfig::default();
        // デフォルト 0.5
        assert!((cc.clamped_faint_opacity() - 0.5).abs() < f32::EPSILON, "デフォルトは 0.5");
        // 負数 → 0.0
        cc.faint_opacity = -0.1;
        assert!((cc.clamped_faint_opacity() - 0.0).abs() < f32::EPSILON, "負数は 0.0");
        // 2.0 → 1.0
        cc.faint_opacity = 2.0;
        assert!((cc.clamped_faint_opacity() - 1.0).abs() < f32::EPSILON, "2.0 は 1.0");
        // NaN → 0.5
        cc.faint_opacity = f32::NAN;
        assert!((cc.clamped_faint_opacity() - 0.5).abs() < f32::EPSILON, "NaN は 0.5");
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

    /// 全テーマで ansi_palette が 16 要素あり、alpha=1.0 であることを検証。
    #[test]
    fn all_themes_ansi_palette_len_and_alpha() {
        for theme in ThemeName::all() {
            let c = ResolvedColors::from_theme(theme);
            assert_eq!(c.ansi_palette.len(), 16, "{theme:?}: ansi_palette.len()");
            for (i, color) in c.ansi_palette.iter().enumerate() {
                assert!(
                    (color[3] - 1.0).abs() < f32::EPSILON,
                    "{theme:?}: ansi_palette[{i}].alpha != 1.0"
                );
            }
        }
    }

    /// palette_generate = false のとき from_color_config は from_theme と同じ結果を返す。
    #[test]
    fn from_color_config_without_generate_equals_from_theme() {
        for theme in ThemeName::all() {
            let config = ColorConfig { theme: theme.clone(), ..ColorConfig::default() };
            let from_config = ResolvedColors::from_color_config(&config);
            let from_theme = ResolvedColors::from_theme(theme);
            assert_eq!(
                from_config.background, from_theme.background,
                "{theme:?}: background mismatch"
            );
            assert_eq!(
                from_config.ansi_palette, from_theme.ansi_palette,
                "{theme:?}: ansi_palette mismatch"
            );
        }
    }

    /// palette_generate = true のとき 16色が生成され、[0] と [15] が bg/fg に近い。
    #[test]
    fn palette_generate_produces_16_colors() {
        let config = ColorConfig {
            theme: ThemeName::CatppuccinMocha,
            palette_generate: true,
            ..ColorConfig::default()
        };
        let resolved = ResolvedColors::from_color_config(&config);
        assert_eq!(resolved.ansi_palette.len(), 16);
        // [15] は fg に等しい
        assert_eq!(resolved.ansi_palette[15], resolved.foreground);
        // alpha がすべて 1.0
        for (i, color) in resolved.ansi_palette.iter().enumerate() {
            assert!((color[3] - 1.0).abs() < f32::EPSILON, "generated palette[{i}].alpha != 1.0");
        }
    }

    /// hsl_to_rgb(rgb_to_hsl(r,g,b)) がほぼ元の値を返すことを検証（ラウンドトリップ）。
    #[test]
    fn hsl_roundtrip() {
        let cases = [(0.5, 0.2, 0.8), (1.0, 0.0, 0.0), (0.0, 1.0, 0.0), (0.0, 0.0, 1.0)];
        for (r, g, b) in cases {
            let (h, s, l) = rgb_to_hsl(r, g, b);
            let (r2, g2, b2) = hsl_to_rgb(h, s, l);
            assert!((r - r2).abs() < 0.001, "r: {r} → {r2}");
            assert!((g - g2).abs() < 0.001, "g: {g} → {g2}");
            assert!((b - b2).abs() < 0.001, "b: {b} → {b2}");
        }
    }
}
