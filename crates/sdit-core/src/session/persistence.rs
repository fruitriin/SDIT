//! セッション永続化 — アプリケーション状態の保存・復元。
//!
//! PTY の中身は保存しない。各セッションの cwd のみを保存し、
//! 起動時にその cwd で新しい PTY セッションを立ち上げる。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// ウィンドウジオメトリのスナップショット。
///
/// `width`/`height` は論理ピクセル（DPI 非依存）、`x`/`y` は物理ピクセル（`outer_position`）。
/// DPI が変わった場合でも論理サイズは OS 側で適切にスケーリングされるため、
/// 物理座標との混在は意図的な設計。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowGeometry {
    /// 論理幅（LogicalSize）。
    pub width: f64,
    /// 論理高さ（LogicalSize）。
    pub height: f64,
    /// 物理X座標（`outer_position`）。
    pub x: i32,
    /// 物理Y座標（`outer_position`）。
    pub y: i32,
}

/// ジオメトリバリデーションの定数。
const MIN_WINDOW_SIZE: f64 = 100.0;
const MAX_WINDOW_SIZE: f64 = 16384.0;
const MAX_WINDOW_POS: i32 = 65536;
const MIN_WINDOW_POS: i32 = -16384;

impl WindowGeometry {
    /// 不正値をデフォルトにクランプしたジオメトリを返す。
    ///
    /// - NaN / Infinity / 極小値 / 極大値はデフォルト（800×600）にフォールバック
    /// - 座標は合理的な範囲にクランプ（マルチモニタ対応）
    #[must_use]
    pub fn validated(self) -> Self {
        let width = if self.width.is_finite() && self.width >= MIN_WINDOW_SIZE {
            self.width.min(MAX_WINDOW_SIZE)
        } else {
            800.0
        };
        let height = if self.height.is_finite() && self.height >= MIN_WINDOW_SIZE {
            self.height.min(MAX_WINDOW_SIZE)
        } else {
            600.0
        };
        let x = self.x.clamp(MIN_WINDOW_POS, MAX_WINDOW_POS);
        let y = self.y.clamp(MIN_WINDOW_POS, MAX_WINDOW_POS);
        Self { width, height, x, y }
    }
}

/// アプリケーション全体のスナップショット。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppSnapshot {
    /// 保存されたセッション一覧。
    #[serde(default)]
    pub sessions: Vec<SessionSnapshot>,
    /// 保存されたウィンドウジオメトリ一覧。
    ///
    /// 後方互換: 古い session.toml に windows フィールドがなくても読める。
    #[serde(default)]
    pub windows: Vec<WindowGeometry>,
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
            windows: vec![],
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

    // -----------------------------------------------------------------------
    // WindowGeometry テスト
    // -----------------------------------------------------------------------

    #[test]
    fn window_geometry_roundtrip() {
        let dir = PathBuf::from("tmp/test-persistence-geometry");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("session.toml");

        let snap = AppSnapshot {
            sessions: vec![SessionSnapshot { cwd: PathBuf::from("/home/user") }],
            windows: vec![
                WindowGeometry { width: 800.0, height: 600.0, x: 100, y: 200 },
                WindowGeometry { width: 1024.0, height: 768.0, x: 300, y: 400 },
            ],
        };

        snap.save(&path).expect("save failed");
        let loaded = AppSnapshot::load(&path);

        assert_eq!(loaded.windows.len(), 2);
        assert!((loaded.windows[0].width - 800.0).abs() < f64::EPSILON);
        assert!((loaded.windows[0].height - 600.0).abs() < f64::EPSILON);
        assert_eq!(loaded.windows[0].x, 100);
        assert_eq!(loaded.windows[0].y, 200);
        assert!((loaded.windows[1].width - 1024.0).abs() < f64::EPSILON);
        assert!((loaded.windows[1].height - 768.0).abs() < f64::EPSILON);
        assert_eq!(loaded.windows[1].x, 300);
        assert_eq!(loaded.windows[1].y, 400);

        // クリーンアップ
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn backward_compat_no_windows_field() {
        // windows フィールドなしの古い TOML を load してもエラーにならない
        let dir = PathBuf::from("tmp/test-persistence-compat");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("session.toml");

        // windows フィールドを含まない旧形式の TOML
        let old_toml = r#"[[sessions]]
cwd = "/home/user"
"#;
        std::fs::write(&path, old_toml).expect("write failed");

        let snap = AppSnapshot::load(&path);
        // windows は空のベクタになる
        assert!(snap.windows.is_empty(), "windows should be empty for old format");
        // sessions は正しく読み込まれる
        assert_eq!(snap.sessions.len(), 1);
        assert_eq!(snap.sessions[0].cwd, PathBuf::from("/home/user"));

        // クリーンアップ
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn validated_clamps_invalid_geometry() {
        // ゼロサイズ → デフォルト
        let geom = WindowGeometry { width: 0.0, height: 0.0, x: 0, y: 0 }.validated();
        assert!((geom.width - 800.0).abs() < f64::EPSILON);
        assert!((geom.height - 600.0).abs() < f64::EPSILON);

        // 負値 → デフォルト
        let geom = WindowGeometry { width: -100.0, height: -50.0, x: 0, y: 0 }.validated();
        assert!((geom.width - 800.0).abs() < f64::EPSILON);
        assert!((geom.height - 600.0).abs() < f64::EPSILON);

        // NaN / Infinity → デフォルト
        let geom =
            WindowGeometry { width: f64::NAN, height: f64::INFINITY, x: 0, y: 0 }.validated();
        assert!((geom.width - 800.0).abs() < f64::EPSILON);
        assert!((geom.height - 600.0).abs() < f64::EPSILON);

        // 極大値 → クランプ
        let geom = WindowGeometry { width: 99999.0, height: 99999.0, x: 0, y: 0 }.validated();
        assert!((geom.width - 16384.0).abs() < f64::EPSILON);
        assert!((geom.height - 16384.0).abs() < f64::EPSILON);

        // 極端な座標 → クランプ
        let geom = WindowGeometry { width: 800.0, height: 600.0, x: -99999, y: 99999 }.validated();
        assert_eq!(geom.x, -16384);
        assert_eq!(geom.y, 65536);

        // 正常値はそのまま
        let geom = WindowGeometry { width: 1024.0, height: 768.0, x: 100, y: 200 }.validated();
        assert!((geom.width - 1024.0).abs() < f64::EPSILON);
        assert!((geom.height - 768.0).abs() < f64::EPSILON);
        assert_eq!(geom.x, 100);
        assert_eq!(geom.y, 200);
    }

    #[test]
    fn empty_windows_list_roundtrip() {
        let dir = PathBuf::from("tmp/test-persistence-empty-windows");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("session.toml");

        // windows が空の AppSnapshot を save/load
        let snap = AppSnapshot { sessions: vec![], windows: vec![] };

        snap.save(&path).expect("save failed");
        let loaded = AppSnapshot::load(&path);
        assert!(loaded.windows.is_empty());
        assert!(loaded.sessions.is_empty());

        // クリーンアップ
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
