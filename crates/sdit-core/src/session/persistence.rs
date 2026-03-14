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
    /// デフォルトの論理幅。
    pub const DEFAULT_WIDTH: f64 = 800.0;
    /// デフォルトの論理高さ。
    pub const DEFAULT_HEIGHT: f64 = 600.0;

    /// 不正値をデフォルトにクランプしたジオメトリを返す。
    ///
    /// - NaN / Infinity / 極小値 / 極大値はデフォルト（800×600）にフォールバック
    /// - 座標は合理的な範囲にクランプ（マルチモニタ対応）
    #[must_use]
    pub fn validated(self) -> Self {
        let width = if self.width.is_finite() && self.width >= MIN_WINDOW_SIZE {
            self.width.min(MAX_WINDOW_SIZE)
        } else {
            Self::DEFAULT_WIDTH
        };
        let height = if self.height.is_finite() && self.height >= MIN_WINDOW_SIZE {
            self.height.min(MAX_WINDOW_SIZE)
        } else {
            Self::DEFAULT_HEIGHT
        };
        let x = self.x.clamp(MIN_WINDOW_POS, MAX_WINDOW_POS);
        let y = self.y.clamp(MIN_WINDOW_POS, MAX_WINDOW_POS);
        Self { width, height, x, y }
    }
}

/// セッション復元情報。ウィンドウごとに保存される各セッションのメタデータ。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRestoreInfo {
    /// ユーザーが設定したカスタムセッション名。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_name: Option<String>,
    /// セッションの作業ディレクトリ（文字列形式）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_directory: Option<String>,
}

impl SessionRestoreInfo {
    /// フィールドの値をバリデーションして安全な値を返す。
    ///
    /// - `working_directory` が 4096 バイトを超える場合は `None` にする
    /// - `custom_name` が 256 バイトを超える場合は `None` にする
    #[must_use]
    pub fn validated(self) -> Self {
        let working_directory = self.working_directory.filter(|p| p.len() <= 4096);
        let custom_name = self.custom_name.filter(|n| n.len() <= 256);
        Self { custom_name, working_directory }
    }
}

/// ウィンドウのスナップショット（ジオメトリ + セッション一覧）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowSnapshot {
    /// ウィンドウのジオメトリ。
    pub geometry: WindowGeometry,
    /// ウィンドウ内のセッション一覧。
    #[serde(default)]
    pub sessions: Vec<SessionRestoreInfo>,
    /// アクティブなセッションのインデックス（0-indexed）。
    #[serde(default)]
    pub active_session_index: usize,
}

impl WindowSnapshot {
    /// `active_session_index` が `sessions` の範囲内に収まるよう検証する。
    ///
    /// 範囲外の場合は 0 を返す。
    #[must_use]
    pub fn validated_active_index(&self) -> usize {
        if self.sessions.is_empty() || self.active_session_index >= self.sessions.len() {
            0
        } else {
            self.active_session_index
        }
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
    /// 新形式: ウィンドウごとのセッション情報（ジオメトリ + セッション一覧）。
    ///
    /// 後方互換: 古い session.toml に window_sessions フィールドがなくても読める。
    #[serde(default)]
    pub window_sessions: Vec<WindowSnapshot>,
}

/// 1セッション分のスナップショット。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSnapshot {
    /// セッションの作業ディレクトリ。
    pub cwd: PathBuf,
    /// ユーザーが設定したカスタムセッション名。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_name: Option<String>,
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
                SessionSnapshot { cwd: PathBuf::from("/home/user"), custom_name: None },
                SessionSnapshot { cwd: PathBuf::from("/tmp"), custom_name: None },
            ],
            ..Default::default()
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

        let win1 = WindowGeometry { width: 800.0, height: 600.0, x: 100, y: 200 };
        let win2 = WindowGeometry { width: 1024.0, height: 768.0, x: 300, y: 400 };
        let snap = AppSnapshot {
            sessions: vec![SessionSnapshot { cwd: PathBuf::from("/home/user"), custom_name: None }],
            windows: vec![win1.clone(), win2.clone()],
            ..Default::default()
        };

        snap.save(&path).expect("save failed");
        let loaded = AppSnapshot::load(&path);

        assert_eq!(loaded.windows.len(), 2, "should load 2 windows");
        assert!((loaded.windows[0].width - win1.width).abs() < f64::EPSILON, "win1 width");
        assert!((loaded.windows[0].height - win1.height).abs() < f64::EPSILON, "win1 height");
        assert_eq!(loaded.windows[0].x, win1.x, "win1 x");
        assert_eq!(loaded.windows[0].y, win1.y, "win1 y");
        assert!((loaded.windows[1].width - win2.width).abs() < f64::EPSILON, "win2 width");
        assert!((loaded.windows[1].height - win2.height).abs() < f64::EPSILON, "win2 height");
        assert_eq!(loaded.windows[1].x, win2.x, "win2 x");
        assert_eq!(loaded.windows[1].y, win2.y, "win2 y");

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
        assert!(
            (geom.width - WindowGeometry::DEFAULT_WIDTH).abs() < f64::EPSILON,
            "zero width → default"
        );
        assert!(
            (geom.height - WindowGeometry::DEFAULT_HEIGHT).abs() < f64::EPSILON,
            "zero height → default"
        );

        // 負値 → デフォルト（MIN_WINDOW_SIZE 未満なのでフォールバック）
        let invalid_w = -100.0_f64;
        let invalid_h = -50.0_f64;
        let geom = WindowGeometry { width: invalid_w, height: invalid_h, x: 0, y: 0 }.validated();
        assert!(
            (geom.width - WindowGeometry::DEFAULT_WIDTH).abs() < f64::EPSILON,
            "negative width → default"
        );
        assert!(
            (geom.height - WindowGeometry::DEFAULT_HEIGHT).abs() < f64::EPSILON,
            "negative height → default"
        );

        // NaN / Infinity → デフォルト
        let geom =
            WindowGeometry { width: f64::NAN, height: f64::INFINITY, x: 0, y: 0 }.validated();
        assert!(
            (geom.width - WindowGeometry::DEFAULT_WIDTH).abs() < f64::EPSILON,
            "NaN width → default"
        );
        assert!(
            (geom.height - WindowGeometry::DEFAULT_HEIGHT).abs() < f64::EPSILON,
            "Inf height → default"
        );

        // MAX_WINDOW_SIZE 超 → クランプ
        let over_max = MAX_WINDOW_SIZE + 1.0;
        let geom = WindowGeometry { width: over_max, height: over_max, x: 0, y: 0 }.validated();
        assert!(
            (geom.width - MAX_WINDOW_SIZE).abs() < f64::EPSILON,
            "over-max width should clamp to MAX_WINDOW_SIZE"
        );
        assert!(
            (geom.height - MAX_WINDOW_SIZE).abs() < f64::EPSILON,
            "over-max height should clamp to MAX_WINDOW_SIZE"
        );

        // 極端な座標 → クランプ
        let geom = WindowGeometry {
            width: WindowGeometry::DEFAULT_WIDTH,
            height: WindowGeometry::DEFAULT_HEIGHT,
            x: MIN_WINDOW_POS - 1,
            y: MAX_WINDOW_POS + 1,
        }
        .validated();
        assert_eq!(geom.x, MIN_WINDOW_POS, "x below MIN should clamp");
        assert_eq!(geom.y, MAX_WINDOW_POS, "y above MAX should clamp");

        // 正常値はそのまま（valid_w, valid_h は MIN/MAX の範囲内の代表値）
        let valid_w = 1024.0_f64;
        let valid_h = 768.0_f64;
        let valid_x = 100_i32;
        let valid_y = 200_i32;
        let geom =
            WindowGeometry { width: valid_w, height: valid_h, x: valid_x, y: valid_y }.validated();
        assert!((geom.width - valid_w).abs() < f64::EPSILON, "valid width should pass through");
        assert!((geom.height - valid_h).abs() < f64::EPSILON, "valid height should pass through");
        assert_eq!(geom.x, valid_x, "valid x should pass through");
        assert_eq!(geom.y, valid_y, "valid y should pass through");
    }

    #[test]
    fn empty_windows_list_roundtrip() {
        let dir = PathBuf::from("tmp/test-persistence-empty-windows");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("session.toml");

        // windows が空の AppSnapshot を save/load
        let snap = AppSnapshot { sessions: vec![], ..Default::default() };

        snap.save(&path).expect("save failed");
        let loaded = AppSnapshot::load(&path);
        assert!(loaded.windows.is_empty());
        assert!(loaded.sessions.is_empty());

        // クリーンアップ
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir_all(&dir);
    }

    // -----------------------------------------------------------------------
    // SessionSnapshot custom_name テスト
    // -----------------------------------------------------------------------

    #[test]
    fn session_snapshot_custom_name_roundtrip() {
        let dir = PathBuf::from("tmp/test-persistence-custom-name");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("session.toml");

        let snap = AppSnapshot {
            sessions: vec![
                SessionSnapshot {
                    cwd: PathBuf::from("/home/user"),
                    custom_name: Some("My Shell".to_owned()),
                },
                SessionSnapshot { cwd: PathBuf::from("/tmp"), custom_name: None },
            ],
            ..Default::default()
        };

        snap.save(&path).expect("save failed");
        let loaded = AppSnapshot::load(&path);
        assert_eq!(loaded.sessions.len(), 2);
        assert_eq!(loaded.sessions[0].custom_name, Some("My Shell".to_owned()));
        assert!(loaded.sessions[1].custom_name.is_none());

        // クリーンアップ
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn session_snapshot_backward_compat_no_custom_name() {
        // custom_name フィールドなしの旧形式 TOML を読み込んでも None になる
        let dir = PathBuf::from("tmp/test-persistence-compat-name");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("session.toml");

        let old_toml = r#"[[sessions]]
cwd = "/home/user"
"#;
        std::fs::write(&path, old_toml).expect("write failed");
        let snap = AppSnapshot::load(&path);
        assert_eq!(snap.sessions.len(), 1);
        assert!(snap.sessions[0].custom_name.is_none());

        // クリーンアップ
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
