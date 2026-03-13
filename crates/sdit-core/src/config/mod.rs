pub mod color;
pub mod font;
pub mod keybinds;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use self::color::ColorConfig;
use self::font::FontConfig;
use self::keybinds::KeybindConfig;
use crate::terminal::CursorStyle;

/// カーソルスタイルの設定値（serde 用）。
///
/// `CursorStyle` とは別に定義し、TOML 文字列との相互変換を担う。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CursorStyleConfig {
    /// ブロックカーソル（デフォルト）。
    #[default]
    Block,
    /// アンダーラインカーソル。
    Underline,
    /// バーカーソル。
    Bar,
}

impl From<CursorStyleConfig> for CursorStyle {
    fn from(c: CursorStyleConfig) -> Self {
        match c {
            CursorStyleConfig::Block => CursorStyle::Block,
            CursorStyleConfig::Underline => CursorStyle::Underline,
            CursorStyleConfig::Bar => CursorStyle::Bar,
        }
    }
}

/// カーソル設定。
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct CursorConfig {
    /// カーソルスタイル: "block" (デフォルト), "underline", "bar"
    pub style: CursorStyleConfig,
    /// カーソル点滅を有効にする（デフォルト: false）。
    pub blinking: bool,
    /// カーソル色（hex 文字列、例: "#ff6600"）。None ならテーマ前景色を使用。
    pub color: Option<String>,
}

/// macOS の Option キーを Alt として扱うかどうかの設定。
///
/// readline ショートカット（Alt+B/F/D 等）を使用するのに必要。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OptionAsAlt {
    /// 左 Option キーのみ Alt として扱う。
    #[serde(alias = "left")]
    OnlyLeft,
    /// 右 Option キーのみ Alt として扱う。
    #[serde(alias = "right")]
    OnlyRight,
    /// 両方の Option キーを Alt として扱う。
    Both,
    /// Option キーを通常通り扱う（デフォルト）。
    #[serde(alias = "none")]
    None,
}

impl Default for OptionAsAlt {
    fn default() -> Self {
        Self::None
    }
}

/// ウィンドウ外観の設定。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct WindowConfig {
    /// ウィンドウ背景の不透明度（0.0 = 完全透明、1.0 = 不透明）。
    pub opacity: f32,
    /// macOS でウィンドウ背景にブラーエフェクトを適用する。
    pub blur: bool,
    /// グリッドとウィンドウ左右端の余白（ピクセル）。デフォルト: 0、最大: 200。
    pub padding_x: u16,
    /// グリッドとウィンドウ上下端の余白（ピクセル）。デフォルト: 0、最大: 200。
    pub padding_y: u16,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self { opacity: 1.0, blur: false, padding_x: 0, padding_y: 0 }
    }
}

impl WindowConfig {
    /// opacity を安全な範囲にクランプする。
    ///
    /// NaN や Inf が渡された場合はデフォルト値 1.0 を返す。
    pub fn clamped_opacity(&self) -> f32 {
        if self.opacity.is_finite() { self.opacity.clamp(0.0, 1.0) } else { 1.0 }
    }

    /// `padding_x` を安全な範囲（0〜200 ピクセル）にクランプする。
    pub fn clamped_padding_x(&self) -> u16 {
        self.padding_x.min(200)
    }

    /// `padding_y` を安全な範囲（0〜200 ピクセル）にクランプする。
    pub fn clamped_padding_y(&self) -> u16 {
        self.padding_y.min(200)
    }
}

/// ベル（BEL 0x07）の設定。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct BellConfig {
    /// ビジュアルベルを有効にする（画面フラッシュ）。
    pub visual: bool,
    /// macOS Dock バウンスを有効にする。
    pub dock_bounce: bool,
    /// ビジュアルベルのフェードアウト時間（ミリ秒）。
    pub duration_ms: u32,
}

impl Default for BellConfig {
    fn default() -> Self {
        Self { visual: true, dock_bounce: true, duration_ms: 150 }
    }
}

impl BellConfig {
    /// duration_ms を安全な範囲にクランプする（0 除算防止 + 長期ループ防止）。
    pub fn clamped_duration_ms(&self) -> u32 {
        self.duration_ms.clamp(1, 5000)
    }
}

/// デスクトップ通知設定。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct NotificationConfig {
    /// デスクトップ通知を有効にする。
    pub enabled: bool,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// ペースト設定。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct PasteConfig {
    /// 複数行テキストのペースト時に確認ダイアログを表示する。
    pub confirm_multiline: bool,
}

impl Default for PasteConfig {
    fn default() -> Self {
        Self { confirm_multiline: true }
    }
}

/// スクロールバック設定。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct ScrollbackConfig {
    /// スクロールバック履歴の最大行数。
    pub lines: u32,
}

impl Default for ScrollbackConfig {
    fn default() -> Self {
        Self { lines: 10_000 }
    }
}

impl ScrollbackConfig {
    /// lines を安全な範囲にクランプする（0-1,000,000）。
    pub fn clamped_lines(&self) -> usize {
        (self.lines as usize).min(1_000_000)
    }
}

/// SDIT 設定全体。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    /// フォント設定。
    pub font: FontConfig,
    /// カラー設定。
    pub colors: ColorConfig,
    /// キーバインド設定。
    pub keybinds: KeybindConfig,
    /// macOS での Option キーの扱い。
    ///
    /// readline ショートカット（Alt+B/F/D 等）を有効にするには `"both"` を設定する。
    /// 有効な値: `"only_left"` / `"left"`, `"only_right"` / `"right"`, `"both"`, `"none"`（デフォルト）。
    /// macOS 以外のプラットフォームでは無視される。
    pub option_as_alt: OptionAsAlt,
    /// ベル設定。
    pub bell: BellConfig,
    /// ウィンドウ外観設定。
    pub window: WindowConfig,
    /// ペースト設定。
    pub paste: PasteConfig,
    /// デスクトップ通知設定。
    pub notification: NotificationConfig,
    /// カーソル設定。
    pub cursor: CursorConfig,
    /// スクロールバック設定。
    pub scrollback: ScrollbackConfig,
}

impl Config {
    /// 設定ファイルを読み込む。
    ///
    /// ファイルが存在しない場合はデフォルト設定を返す。
    /// パースエラーの場合はログに警告を出してデフォルト設定を返す。
    pub fn load(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(contents) => match toml::from_str::<Config>(&contents) {
                Ok(mut config) => {
                    config.keybinds.validate();
                    log::info!("Loaded config from {}", path.display());
                    config
                }
                Err(e) => {
                    log::warn!("Config parse error in {}: {e}", path.display());
                    Self::default()
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                log::info!("No config file at {}, using defaults", path.display());
                Self::default()
            }
            Err(e) => {
                log::warn!("Failed to read config {}: {e}", path.display());
                Self::default()
            }
        }
    }

    /// デフォルトの設定ファイルパスを返す。
    ///
    /// `$XDG_CONFIG_HOME/sdit/sdit.toml`（macOS では `~/.config/sdit/sdit.toml`）。
    pub fn default_path() -> PathBuf {
        dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("sdit").join("sdit.toml")
    }

    /// 設定を TOML ファイルに書き出す。
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let content = toml::to_string_pretty(self).map_err(std::io::Error::other)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)
    }

    /// コメント付きの設定テンプレートを生成して保存する。
    ///
    /// 初回起動時や設定ファイル未存在時に呼ばれる。
    /// コメント付きの設定テンプレートを生成し、**ファイルが存在しない場合のみ**書き出す。
    ///
    /// `create_new(true)` で排他的に作成するため、TOCTOU 競合を防ぐ。
    /// ファイルが既に存在する場合は `Ok(())` を返す（何もしない）。
    pub fn save_with_comments(&self, path: &Path) -> std::io::Result<()> {
        use std::io::Write;

        let toml_body = toml::to_string_pretty(self).map_err(std::io::Error::other)?;

        let mut content = String::new();
        content.push_str("# SDIT Terminal Configuration\n");
        content.push_str("# Changes are applied automatically (hot reload).\n");
        content.push_str("#\n");
        content.push_str("# Documentation: https://github.com/user/sdit\n\n");

        // TOML ボディの各行を走査し、セクションヘッダーの前にコメントを挿入する。
        for line in toml_body.lines() {
            if line == "[font]" {
                content.push_str("# ── Font ──────────────────────────────────────────────\n");
                content.push_str("# family: font family name (default: system monospace)\n");
                content.push_str("# size: font size in pixels (1.0 - 200.0, default: 14.0)\n");
                content
                    .push_str("# line_height: line height multiplier (0.5 - 5.0, default: 1.2)\n");
                content.push_str(
                    "# fallback_families: list of fallback font families (e.g. for CJK)\n",
                );
            } else if line == "[colors]" {
                content.push('\n');
                content.push_str("# ── Colors ─────────────────────────────────────────────\n");
                content.push_str("# theme: built-in color theme name\n");
                content.push_str(
                    "#   available: \"catppuccin-mocha\", \"catppuccin-latte\", \"gruvbox-dark\"\n",
                );
            } else if line == "[keybinds]" || line == "[[keybinds]]" {
                content.push('\n');
                content.push_str("# ── Keybinds ────────────────────────────────────────────\n");
                content.push_str(
                    "# Each entry: key, mods (\"super\", \"ctrl\", \"shift\", \"alt\", combined with \"|\"), action\n",
                );
                content
                    .push_str("# Example: key = \"n\", mods = \"super\", action = \"NewWindow\"\n");
            } else if line.starts_with("option_as_alt") {
                content.push('\n');
                content.push_str("# ── macOS Option Key ────────────────────────────────────\n");
                content.push_str("# option_as_alt: treat Option key as Alt for readline shortcuts (Alt+B/F/D etc.)\n");
                content.push_str(
                    "#   values: \"none\" (default), \"both\", \"only_left\" / \"left\", \"only_right\" / \"right\"\n",
                );
            } else if line == "[bell]" {
                content.push('\n');
                content.push_str("# ── Bell ──────────────────────────────────────────────\n");
                content.push_str(
                    "# visual: flash the screen when BEL (0x07) is received (default: true)\n",
                );
                content.push_str("# dock_bounce: bounce the Dock icon when BEL is received while unfocused (default: true, macOS only)\n");
                content.push_str(
                    "# duration_ms: visual bell fade-out duration in milliseconds (default: 150)\n",
                );
            } else if line == "[window]" {
                content.push('\n');
                content.push_str("# ── Window ─────────────────────────────────────────────\n");
                content.push_str(
                    "# opacity: background opacity (0.0 = fully transparent, 1.0 = opaque, default: 1.0)\n",
                );
                content.push_str(
                    "# blur: enable background blur effect (macOS only, default: false)\n",
                );
                content.push_str(
                    "# padding_x: horizontal padding between grid and window edge in pixels (0-200, default: 0)\n",
                );
                content.push_str(
                    "# padding_y: vertical padding between grid and window edge in pixels (0-200, default: 0)\n",
                );
            } else if line == "[paste]" {
                content.push('\n');
                content.push_str("# ── Paste ─────────────────────────────────────────────\n");
                content.push_str("# confirm_multiline: show confirmation dialog when pasting text containing newlines (default: true)\n");
            } else if line == "[notification]" {
                content.push('\n');
                content.push_str("# ── Notification ──────────────────────────────────────\n");
                content.push_str("# enabled: show desktop notifications from OSC 9/99 sequences (default: true)\n");
            } else if line == "[cursor]" {
                content.push('\n');
                content.push_str("# ── Cursor ─────────────────────────────────────────────\n");
                content.push_str(
                    "# style: cursor shape: \"block\" (default), \"underline\", \"bar\"\n",
                );
                content.push_str("# blinking: enable cursor blinking (default: false)\n");
                content.push_str("# color: cursor color as hex string (e.g. \"#ff6600\"); omit to use theme foreground\n");
            } else if line == "[scrollback]" {
                content.push('\n');
                content.push_str("# ── Scrollback ──────────────────────────────────────────\n");
                content.push_str(
                    "# lines: maximum number of scrollback lines (default: 10000, range: 0-1000000)\n",
                );
            }
            content.push_str(line);
            content.push('\n');
        }

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        match std::fs::OpenOptions::new().write(true).create_new(true).open(path) {
            Ok(mut file) => file.write_all(content.as_bytes()),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        let config = Config::default();
        assert!(!config.font.family.is_empty());
        assert!(config.font.size > 0.0);
    }

    #[test]
    fn option_as_alt_default_is_none() {
        let config = Config::default();
        assert_eq!(config.option_as_alt, OptionAsAlt::None);
    }

    #[test]
    fn option_as_alt_deserialize_canonical() {
        let cases: &[(&str, OptionAsAlt)] = &[
            (r#"option_as_alt = "none""#, OptionAsAlt::None),
            (r#"option_as_alt = "both""#, OptionAsAlt::Both),
            (r#"option_as_alt = "only_left""#, OptionAsAlt::OnlyLeft),
            (r#"option_as_alt = "only_right""#, OptionAsAlt::OnlyRight),
        ];
        for (toml_str, expected) in cases {
            let config: Config = toml::from_str(toml_str).unwrap();
            assert_eq!(config.option_as_alt, *expected, "failed for: {toml_str}");
        }
    }

    #[test]
    fn option_as_alt_deserialize_alias() {
        let left: Config = toml::from_str(r#"option_as_alt = "left""#).unwrap();
        assert_eq!(left.option_as_alt, OptionAsAlt::OnlyLeft);

        let right: Config = toml::from_str(r#"option_as_alt = "right""#).unwrap();
        assert_eq!(right.option_as_alt, OptionAsAlt::OnlyRight);
    }

    #[test]
    fn option_as_alt_serialize() {
        let config = Config { option_as_alt: OptionAsAlt::Both, ..Default::default() };
        let serialized = toml::to_string(&config).unwrap();
        assert!(
            serialized.contains("option_as_alt = \"both\""),
            "expected serialized form: {serialized}"
        );
    }

    #[test]
    fn load_nonexistent_returns_default() {
        let config = Config::load(Path::new("/nonexistent/path/sdit.toml"));
        assert!(!config.font.family.is_empty());
        assert!((config.font.size - 14.0).abs() < f32::EPSILON);
    }

    #[test]
    fn deserialize_full_config() {
        let toml_str = r#"
[font]
family = "JetBrains Mono"
size = 16.0
line_height = 1.3
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.font.family, "JetBrains Mono");
        assert!((config.font.size - 16.0).abs() < f32::EPSILON);
        assert!((config.font.line_height - 1.3).abs() < f32::EPSILON);
    }

    #[test]
    fn deserialize_empty_uses_defaults() {
        let config: Config = toml::from_str("").unwrap();
        assert!(!config.font.family.is_empty());
        assert!((config.font.size - 14.0).abs() < f32::EPSILON);
    }

    #[test]
    fn default_path_not_empty() {
        let path = Config::default_path();
        assert!(!path.as_os_str().is_empty());
    }

    #[test]
    fn config_save_and_load_roundtrip() {
        let config = Config::default();
        let path = std::path::PathBuf::from("tmp/test-config-roundtrip.toml");
        std::fs::create_dir_all("tmp").expect("tmp dir");
        config.save(&path).expect("save failed");
        let loaded = Config::load(&path);
        assert!(
            (loaded.font.size - config.font.size).abs() < f32::EPSILON,
            "font.size mismatch: {} vs {}",
            loaded.font.size,
            config.font.size
        );
        assert_eq!(loaded.font.family, config.font.family);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn bell_config_default() {
        let bell = BellConfig::default();
        assert!(bell.visual);
        assert!(bell.dock_bounce);
        assert_eq!(bell.duration_ms, 150);
    }

    #[test]
    fn bell_config_deserialize() {
        let toml_str = "[bell]\nvisual = false\ndock_bounce = false\nduration_ms = 200\n";
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(!config.bell.visual);
        assert!(!config.bell.dock_bounce);
        assert_eq!(config.bell.duration_ms, 200);
    }

    #[test]
    fn bell_config_partial_deserialize() {
        // 部分指定のとき残りはデフォルト補完
        let toml_str = "[bell]\nduration_ms = 300\n";
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(config.bell.visual); // default: true
        assert!(config.bell.dock_bounce); // default: true
        assert_eq!(config.bell.duration_ms, 300);
    }

    #[test]
    fn bell_duration_clamp_zero() {
        let bell = BellConfig { duration_ms: 0, ..Default::default() };
        assert_eq!(bell.clamped_duration_ms(), 1);
    }

    #[test]
    fn bell_duration_clamp_max() {
        let bell = BellConfig { duration_ms: 999_999, ..Default::default() };
        assert_eq!(bell.clamped_duration_ms(), 5000);
    }

    #[test]
    fn window_config_default() {
        let wc = WindowConfig::default();
        assert!((wc.opacity - 1.0).abs() < f32::EPSILON);
        assert!(!wc.blur);
        assert_eq!(wc.padding_x, 0);
        assert_eq!(wc.padding_y, 0);
    }

    #[test]
    fn window_config_deserialize() {
        let toml_str = "[window]\nopacity = 0.8\nblur = true\n";
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!((config.window.opacity - 0.8).abs() < f32::EPSILON);
        assert!(config.window.blur);
    }

    #[test]
    fn window_config_padding_deserialize() {
        let toml_str = "[window]\npadding_x = 8\npadding_y = 4\n";
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.window.padding_x, 8);
        assert_eq!(config.window.padding_y, 4);
    }

    #[test]
    fn window_config_partial_deserialize() {
        let toml_str = "[window]\nopacity = 0.5\n";
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!((config.window.opacity - 0.5).abs() < f32::EPSILON);
        assert!(!config.window.blur); // default
    }

    #[test]
    fn window_opacity_clamp() {
        let wc = WindowConfig { opacity: -0.5, ..Default::default() };
        assert!((wc.clamped_opacity() - 0.0).abs() < f32::EPSILON);
        let wc = WindowConfig { opacity: 2.0, ..Default::default() };
        assert!((wc.clamped_opacity() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn window_padding_clamp() {
        // 200 を超える値は 200 にクランプされる
        let wc = WindowConfig { padding_x: 500, padding_y: 300, ..Default::default() };
        assert_eq!(wc.clamped_padding_x(), 200);
        assert_eq!(wc.clamped_padding_y(), 200);
        // 範囲内の値はそのまま
        let wc2 = WindowConfig { padding_x: 8, padding_y: 4, ..Default::default() };
        assert_eq!(wc2.clamped_padding_x(), 8);
        assert_eq!(wc2.clamped_padding_y(), 4);
    }

    #[test]
    fn window_opacity_clamp_nan_inf() {
        let wc = WindowConfig { opacity: f32::NAN, ..Default::default() };
        assert!((wc.clamped_opacity() - 1.0).abs() < f32::EPSILON);
        let wc = WindowConfig { opacity: f32::INFINITY, ..Default::default() };
        assert!((wc.clamped_opacity() - 1.0).abs() < f32::EPSILON);
        let wc = WindowConfig { opacity: f32::NEG_INFINITY, ..Default::default() };
        assert!((wc.clamped_opacity() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn paste_config_default() {
        let pc = PasteConfig::default();
        assert!(pc.confirm_multiline);
    }

    #[test]
    fn paste_config_deserialize() {
        let toml_str = "[paste]\nconfirm_multiline = false\n";
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(!config.paste.confirm_multiline);
    }

    #[test]
    fn notification_config_default() {
        let nc = NotificationConfig::default();
        assert!(nc.enabled);
    }

    #[test]
    fn notification_config_deserialize() {
        let toml_str = "[notification]\nenabled = false\n";
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(!config.notification.enabled);
    }

    #[test]
    fn config_save_with_comments_is_parseable() {
        let config = Config::default();
        let path = std::path::PathBuf::from("tmp/test-config-comments.toml");
        std::fs::create_dir_all("tmp").expect("tmp dir");
        // create_new(true) を使うため、既存ファイルを先に削除する
        let _ = std::fs::remove_file(&path);
        config.save_with_comments(&path).expect("save failed");
        let loaded = Config::load(&path);
        assert!(
            (loaded.font.size - config.font.size).abs() < f32::EPSILON,
            "font.size mismatch after comment-save: {} vs {}",
            loaded.font.size,
            config.font.size
        );
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("# SDIT"), "expected '# SDIT' comment header");
        assert!(content.contains("[font]"), "expected [font] section");
        let _ = std::fs::remove_file(&path);
    }

    // -----------------------------------------------------------------------
    // CursorConfig テスト
    // -----------------------------------------------------------------------

    #[test]
    fn cursor_config_default() {
        let cc = CursorConfig::default();
        assert_eq!(cc.style, CursorStyleConfig::Block);
        assert!(!cc.blinking);
        assert!(cc.color.is_none());
    }

    #[test]
    fn cursor_config_deserialize_full() {
        let toml_str = "[cursor]\nstyle = \"bar\"\nblinking = true\ncolor = \"#ff6600\"\n";
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.cursor.style, CursorStyleConfig::Bar);
        assert!(config.cursor.blinking);
        assert_eq!(config.cursor.color.as_deref(), Some("#ff6600"));
    }

    #[test]
    fn cursor_config_deserialize_partial() {
        let toml_str = "[cursor]\nstyle = \"underline\"\n";
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.cursor.style, CursorStyleConfig::Underline);
        assert!(!config.cursor.blinking); // default
        assert!(config.cursor.color.is_none()); // default
    }

    #[test]
    fn cursor_config_deserialize_empty_uses_defaults() {
        let config: Config = toml::from_str("").unwrap();
        assert_eq!(config.cursor.style, CursorStyleConfig::Block);
        assert!(!config.cursor.blinking);
        assert!(config.cursor.color.is_none());
    }

    #[test]
    fn cursor_style_config_converts_to_cursor_style() {
        use crate::terminal::CursorStyle;
        assert_eq!(CursorStyle::from(CursorStyleConfig::Block), CursorStyle::Block);
        assert_eq!(CursorStyle::from(CursorStyleConfig::Underline), CursorStyle::Underline);
        assert_eq!(CursorStyle::from(CursorStyleConfig::Bar), CursorStyle::Bar);
    }

    // -----------------------------------------------------------------------
    // ScrollbackConfig テスト
    // -----------------------------------------------------------------------

    #[test]
    fn scrollback_config_default() {
        let sc = ScrollbackConfig::default();
        assert_eq!(sc.lines, 10_000);
    }

    #[test]
    fn scrollback_config_deserialize() {
        let toml_str = "[scrollback]\nlines = 50000\n";
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.scrollback.lines, 50_000);
    }

    #[test]
    fn scrollback_clamped_lines_zero() {
        let sc = ScrollbackConfig { lines: 0 };
        assert_eq!(sc.clamped_lines(), 0);
    }

    #[test]
    fn scrollback_clamped_lines_over_max() {
        let sc = ScrollbackConfig { lines: 2_000_000 };
        assert_eq!(sc.clamped_lines(), 1_000_000);
    }

    #[test]
    fn scrollback_config_empty_uses_default() {
        let config: Config = toml::from_str("").unwrap();
        assert_eq!(config.scrollback.lines, 10_000);
    }
}
