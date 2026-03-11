pub mod color;
pub mod font;

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::color::ColorConfig;
use crate::font::FontConfig;

/// SDIT 設定全体。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    /// フォント設定。
    pub font: FontConfig,
    /// カラー設定。
    pub colors: ColorConfig,
}

impl Config {
    /// 設定ファイルを読み込む。
    ///
    /// ファイルが存在しない場合はデフォルト設定を返す。
    /// パースエラーの場合はログに警告を出してデフォルト設定を返す。
    pub fn load(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(contents) => match toml::from_str::<Config>(&contents) {
                Ok(config) => {
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
}
