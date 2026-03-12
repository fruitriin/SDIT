pub mod color;
pub mod font;
pub mod keybinds;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use self::color::ColorConfig;
use self::font::FontConfig;
use self::keybinds::KeybindConfig;

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
            } else if line == "[[keybinds]]" {
                content.push('\n');
                content.push_str("# ── Keybinds ────────────────────────────────────────────\n");
                content.push_str(
                    "# Each entry: key, mods (\"super\", \"ctrl\", \"shift\", \"alt\", combined with \"|\"), action\n",
                );
                content
                    .push_str("# Example: key = \"n\", mods = \"super\", action = \"NewWindow\"\n");
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
}
