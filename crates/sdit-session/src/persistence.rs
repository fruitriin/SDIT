//! セッション永続化 — アプリケーション状態の保存・復元。
//!
//! PTY の中身は保存しない。各セッションの cwd のみを保存し、
//! 起動時にその cwd で新しい PTY セッションを立ち上げる。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// アプリケーション全体のスナップショット。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppSnapshot {
    /// 保存されたセッション一覧。
    #[serde(default)]
    pub sessions: Vec<SessionSnapshot>,
}

/// 1セッション分のスナップショット。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSnapshot {
    /// セッションの作業ディレクトリ。
    pub cwd: PathBuf,
}

impl AppSnapshot {
    /// デフォルトの保存先パスを返す。
    ///
    /// `~/.local/state/sdit/session.toml`
    pub fn default_path() -> PathBuf {
        dirs::state_dir()
            .or_else(dirs::data_local_dir)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("sdit")
            .join("session.toml")
    }

    /// ファイルからスナップショットを読み込む。
    ///
    /// ファイルが存在しない場合や破損している場合はデフォルト値を返す。
    pub fn load(path: &Path) -> Self {
        let Ok(content) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        toml::from_str(&content).unwrap_or_default()
    }

    /// ファイルにスナップショットを保存する。
    ///
    /// 親ディレクトリが存在しない場合は作成する。
    /// アトミック書き込み（一時ファイル + rename）で破損を防ぐ。
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content =
            toml::to_string_pretty(self).map_err(|e| std::io::Error::other(e.to_string()))?;

        // 一時ファイルに書き込んでから rename でアトミックに置換。
        // PID を含めて予測困難な一時ファイル名にし、TOCTOU 攻撃を軽減する。
        let tmp_name = format!(
            "session.{}.{}.tmp",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        let tmp_path = path.with_file_name(tmp_name);
        std::fs::write(&tmp_path, &content)?;
        if let Err(e) = std::fs::rename(&tmp_path, path) {
            // rename 失敗時は一時ファイルをクリーンアップ
            let _ = std::fs::remove_file(&tmp_path);
            return Err(e);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_snapshot_is_empty() {
        let snap = AppSnapshot::default();
        assert!(snap.sessions.is_empty());
    }

    #[test]
    fn load_nonexistent_returns_default() {
        let snap = AppSnapshot::load(Path::new("/nonexistent/path/session.toml"));
        assert!(snap.sessions.is_empty());
    }

    #[test]
    fn roundtrip_save_load() {
        let dir = PathBuf::from("tmp/test-persistence");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("session.toml");

        let snap = AppSnapshot {
            sessions: vec![
                SessionSnapshot { cwd: PathBuf::from("/home/user") },
                SessionSnapshot { cwd: PathBuf::from("/tmp") },
            ],
        };

        snap.save(&path).expect("save failed");
        let loaded = AppSnapshot::load(&path);
        assert_eq!(loaded.sessions.len(), 2);
        assert_eq!(loaded.sessions[0].cwd, PathBuf::from("/home/user"));
        assert_eq!(loaded.sessions[1].cwd, PathBuf::from("/tmp"));

        // クリーンアップ
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_corrupted_returns_default() {
        let dir = PathBuf::from("tmp/test-persistence-corrupt");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("session.toml");

        std::fs::write(&path, "not valid toml {{{{").expect("write failed");
        let snap = AppSnapshot::load(&path);
        assert!(snap.sessions.is_empty());

        // クリーンアップ
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn default_path_exists() {
        let path = AppSnapshot::default_path();
        assert!(path.to_string_lossy().contains("sdit"));
        assert!(path.to_string_lossy().ends_with("session.toml"));
    }
}
