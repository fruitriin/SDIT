use std::sync::Arc;

use winit::event_loop::ActiveEventLoop;
use winit::window::{Fullscreen, Window, WindowId, WindowLevel};

#[cfg(target_os = "macos")]
use winit::platform::macos::{OptionAsAlt as WinitOptionAsAlt, WindowExtMacOS};

use sdit_core::config::{BackgroundImageFit, Decorations, StartupMode};
use sdit_core::pty::PtySize;
use sdit_core::render::atlas::Atlas;
use sdit_core::render::pipeline::{BackgroundPipeline, CellPipeline, GpuContext};
use sdit_core::session::{
    AppSnapshot, SessionRestoreInfo, SidebarState, WindowGeometry, WindowSnapshot,
};

use crate::app::{SditApp, VisualBell, WindowState};
use crate::window::calc_grid_size;

/// 画像の最大許容寸法（ピクセル）。4096x4096 を超えるとメモリ枯渇の恐れがある。
const MAX_DIMENSION: u32 = 4096;

/// 背景画像を読み込んで RGBA8 ピクセルデータと寸法を返す。
///
/// - パスが `~` で始まる場合はホームディレクトリに展開する
/// - `~` 展開後に `canonicalize()` してホームディレクトリ内であることを確認する
/// - `..` を含むパスを拒否する（パストラバーサル防止）
/// - ファイルが見つからない場合は `None` を返して warn ログを出す
/// - ファイルサイズが 10MB を超える場合はスキップする
/// - パス内に制御文字が含まれる場合はスキップする
/// - 画像の寸法が `MAX_DIMENSION` を超える場合はスキップする
pub(crate) fn load_background_image(path_str: &str) -> Option<(Vec<u8>, u32, u32)> {
    // 制御文字チェック
    if path_str.chars().any(|c| c.is_control()) {
        log::warn!("background_image: path contains control characters, skipping");
        return None;
    }

    // `..` コンポーネントを含むパスを拒否する（パストラバーサル防止）
    let raw_path = std::path::Path::new(path_str);
    for component in raw_path.components() {
        if component == std::path::Component::ParentDir {
            log::warn!("background_image: path traversal detected (\"..\" component), skipping");
            return None;
        }
    }

    // `~` をホームディレクトリに展開
    let expanded: std::path::PathBuf = if let Some(rest) = path_str.strip_prefix("~/") {
        let home = match dirs::home_dir() {
            Some(h) => h,
            None => {
                log::warn!("background_image: cannot determine home directory, skipping");
                return None;
            }
        };
        home.join(rest)
    } else if path_str == "~" {
        match dirs::home_dir() {
            Some(h) => h,
            None => {
                log::warn!("background_image: cannot determine home directory, skipping");
                return None;
            }
        }
    } else {
        std::path::PathBuf::from(path_str)
    };

    // canonicalize して実際のパスを解決する（シンボリックリンク展開後もチェック）
    let canonical = match expanded.canonicalize() {
        Ok(p) => p,
        Err(e) => {
            log::warn!("background_image: cannot resolve path {}: {e}", expanded.display());
            return None;
        }
    };

    // `~` で始まるパスはホームディレクトリ内であることを確認する
    if path_str.starts_with("~/") || path_str == "~" {
        if let Some(home) = dirs::home_dir() {
            if let Ok(canonical_home) = home.canonicalize() {
                if !canonical.starts_with(&canonical_home) {
                    log::warn!(
                        "background_image: path escapes home directory, skipping: {}",
                        canonical.display()
                    );
                    return None;
                }
            }
        }
    }

    // ファイルを先に open してから metadata でサイズチェックする（TOCTOU 防止）
    const MAX_SIZE: u64 = 10 * 1024 * 1024;
    let file = match std::fs::File::open(&canonical) {
        Ok(f) => f,
        Err(e) => {
            log::warn!("background_image: cannot open file {}: {e}", canonical.display());
            return None;
        }
    };
    match file.metadata() {
        Ok(meta) if meta.len() > MAX_SIZE => {
            log::warn!(
                "background_image: file too large ({} bytes > 10MB), skipping: {}",
                meta.len(),
                canonical.display()
            );
            return None;
        }
        Err(e) => {
            log::warn!("background_image: cannot read file metadata {}: {e}", canonical.display());
            return None;
        }
        _ => {}
    }
    drop(file); // ImageReader が canonical を直接 open するため、ここで閉じる

    // フォーマット推測 → 寸法チェック → デコードの順で処理する
    let reader = match image::ImageReader::open(&canonical) {
        Ok(r) => r,
        Err(e) => {
            log::warn!("background_image: failed to open {}: {e}", canonical.display());
            return None;
        }
    };
    let reader = match reader.with_guessed_format() {
        Ok(r) => r,
        Err(e) => {
            log::warn!("background_image: failed to guess format for {}: {e}", canonical.display());
            return None;
        }
    };

    // 寸法チェック（デコード前）
    match reader.into_dimensions() {
        Ok((w, h)) => {
            if w == 0 || h == 0 || w > MAX_DIMENSION || h > MAX_DIMENSION {
                log::warn!(
                    "background_image: image dimensions {}x{} exceed limit (max {}), skipping: {}",
                    w,
                    h,
                    MAX_DIMENSION,
                    canonical.display()
                );
                return None;
            }
        }
        Err(e) => {
            log::warn!(
                "background_image: failed to read dimensions for {}: {e}",
                canonical.display()
            );
            return None;
        }
    }

    // 再度 open してデコードする（into_dimensions で reader を消費するため）
    match image::open(&canonical) {
        Ok(img) => {
            let rgba = img.into_rgba8();
            let width = rgba.width();
            let height = rgba.height();
            // デコード後も寸法が制限内であることを確認する
            if width == 0 || height == 0 || width > MAX_DIMENSION || height > MAX_DIMENSION {
                log::warn!(
                    "background_image: decoded dimensions {}x{} exceed limit, skipping",
                    width,
                    height
                );
                return None;
            }
            Some((rgba.into_raw(), width, height))
        }
        Err(e) => {
            log::warn!("background_image: failed to load {}: {e}", canonical.display());
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_background_image_path_traversal_dotdot() {
        // `..` コンポーネントを含むパスは拒否される
        assert!(load_background_image("../foo.png").is_none());
        assert!(load_background_image("../../etc/passwd").is_none());
    }

    #[test]
    fn load_background_image_path_traversal_tilde_escape() {
        // `~/../../etc/passwd` 形式（`~` 展開後にホームを脱出しようとするパス）
        // `..` チェックで先にブロックされる
        assert!(load_background_image("~/../../../etc/passwd").is_none());
    }

    #[test]
    fn load_background_image_control_characters() {
        // パスに制御文字が含まれる場合はスキップ
        assert!(load_background_image("foo\x00bar.png").is_none());
        assert!(load_background_image("foo\nbar.png").is_none());
        assert!(load_background_image("foo\tbar.png").is_none());
    }

    #[test]
    fn load_background_image_nonexistent_file() {
        // 存在しないファイルは None を返す
        assert!(load_background_image("/nonexistent/path/image.png").is_none());
    }
}

/// sdit-core の `OptionAsAlt` を winit の `WinitOptionAsAlt` に変換する。
#[cfg(target_os = "macos")]
pub(crate) fn config_option_as_alt_to_winit(v: sdit_core::config::OptionAsAlt) -> WinitOptionAsAlt {
    match v {
        sdit_core::config::OptionAsAlt::OnlyLeft => WinitOptionAsAlt::OnlyLeft,
        sdit_core::config::OptionAsAlt::OnlyRight => WinitOptionAsAlt::OnlyRight,
        sdit_core::config::OptionAsAlt::Both => WinitOptionAsAlt::Both,
        sdit_core::config::OptionAsAlt::None => WinitOptionAsAlt::None,
    }
}

impl SditApp {
    /// 現在のウィンドウ群のジオメトリを収集する。
    fn collect_window_geometries(&self) -> Vec<WindowGeometry> {
        self.windows
            .values()
            .filter_map(|ws| {
                let size = ws.window.inner_size().to_logical::<f64>(ws.window.scale_factor());
                let pos = ws.window.outer_position().ok()?;
                Some(WindowGeometry { width: size.width, height: size.height, x: pos.x, y: pos.y })
            })
            .collect()
    }

    /// 現在のウィンドウ群のセッション情報を収集する。
    ///
    /// 各ウィンドウのジオメトリ・セッション一覧・アクティブインデックスを収集する。
    fn collect_window_sessions(&self) -> Vec<WindowSnapshot> {
        self.windows
            .values()
            .filter_map(|ws| {
                let size = ws.window.inner_size().to_logical::<f64>(ws.window.scale_factor());
                let pos = ws.window.outer_position().ok()?;
                let geometry =
                    WindowGeometry { width: size.width, height: size.height, x: pos.x, y: pos.y };

                let sessions: Vec<SessionRestoreInfo> = ws
                    .sessions
                    .iter()
                    .map(|&sid| {
                        let session = self.session_mgr.get(sid);
                        SessionRestoreInfo {
                            custom_name: session.as_ref().and_then(|s| s.custom_name.clone()),
                            working_directory: session
                                .as_ref()
                                .and_then(|s| s.cwd.as_ref())
                                .and_then(|p| p.to_str())
                                .map(|s| s.to_owned()),
                        }
                    })
                    .collect();

                Some(WindowSnapshot { geometry, sessions, active_session_index: ws.active_index })
            })
            .collect()
    }

    /// Quick Terminal ウィンドウを生成する（macOS のみ）。
    ///
    /// ボーダーレス + 透明 + AlwaysOnTop のウィンドウを作成し、
    /// 指定された位置・サイズに配置する。
    /// セッションも同時に生成し、`WindowState` に登録する。
    #[cfg(target_os = "macos")]
    pub(crate) fn create_quick_terminal_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) -> Option<winit::window::WindowId> {
        let attrs = Window::default_attributes()
            .with_title("SDIT Quick Terminal")
            .with_decorations(false)
            .with_transparent(true)
            .with_inner_size(winit::dpi::PhysicalSize::new(width, height))
            .with_position(winit::dpi::PhysicalPosition::new(x, y));

        let window = match event_loop.create_window(attrs) {
            Ok(w) => {
                w.set_ime_allowed(true);
                w.set_window_level(WindowLevel::AlwaysOnTop);
                #[cfg(target_os = "macos")]
                w.set_option_as_alt(config_option_as_alt_to_winit(self.config.option_as_alt));
                std::sync::Arc::new(w)
            }
            Err(e) => {
                log::error!("Quick Terminal window creation failed: {e}");
                return None;
            }
        };

        let prefer_wide =
            self.config.window.colorspace == sdit_core::config::WindowColorspace::DisplayP3;
        let gpu = match sdit_core::render::pipeline::GpuContext::new(&window, prefer_wide) {
            Ok(g) => g,
            Err(e) => {
                log::error!("Quick Terminal GPU context creation failed: {e}");
                return None;
            }
        };

        let metrics = *self.font_ctx.metrics();
        let padding_x = f32::from(self.config.window.clamped_padding_x());
        let padding_y = f32::from(self.config.window.clamped_padding_y());
        let (cols, rows) = calc_grid_size(
            (gpu.surface_config.width as f32 - 2.0 * padding_x).max(0.0),
            (gpu.surface_config.height as f32 - 2.0 * padding_y).max(0.0),
            metrics.cell_width,
            metrics.cell_height,
        );

        let Some(session_id) = self.spawn_session_with_cwd(rows, cols, None) else {
            return None;
        };
        let session = self.session_mgr.get(session_id).unwrap();

        let mut atlas = sdit_core::render::atlas::Atlas::new(&gpu.device, 512);
        let cell_size = [metrics.cell_width, metrics.cell_height];
        let surface_size = [gpu.surface_config.width as f32, gpu.surface_config.height as f32];
        let atlas_size_f32 = atlas.size() as f32;

        let state_lock =
            session.term_state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let grid = state_lock.terminal.grid();
        let mut cell_pipeline = sdit_core::render::pipeline::CellPipeline::new(
            &gpu.device,
            gpu.surface_config.format,
            &atlas,
            rows * cols,
        );
        let cursor_col = grid.cursor.point.column.0;
        #[allow(clippy::cast_sign_loss)]
        let cursor_row = grid.cursor.point.line.0 as usize;
        cell_pipeline.update_from_grid(
            &gpu.queue,
            grid,
            &mut self.font_ctx,
            &mut atlas,
            atlas_size_f32,
            cell_size,
            surface_size,
            Some((cursor_col, cursor_row)),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            1.0,
            false, // bold_is_bright: Quick Terminal ではデフォルト
            0.5,   // faint_opacity: Quick Terminal ではデフォルト
        );
        drop(state_lock);
        atlas.upload_if_dirty(&gpu.queue);

        let sidebar_pipeline = sdit_core::render::pipeline::CellPipeline::new(
            &gpu.device,
            gpu.surface_config.format,
            &atlas,
            100,
        );

        let window_id = window.id();
        self.session_to_window.insert(session_id, window_id);
        self.windows.insert(
            window_id,
            crate::app::WindowState {
                window,
                gpu,
                cell_pipeline,
                sidebar_pipeline,
                bg_pipeline: None,
                atlas,
                sessions: vec![session_id],
                active_index: 0,
                sidebar: sdit_core::session::SidebarState::new(),
                visual_bell: crate::app::VisualBell::new(self.config.bell.clamped_duration_ms()),
            },
        );

        log::info!("Created Quick Terminal window {window_id:?} with session {}", session_id.0);
        Some(window_id)
    }

    /// 新しいウィンドウ + セッションを生成する。
    ///
    /// `geometry` が `Some` の場合、指定サイズ・位置でウィンドウを作成する。
    /// `None` の場合はデフォルト（800×600）でカスケード配置する。
    pub(crate) fn create_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        geometry: Option<&WindowGeometry>,
    ) {
        self.create_window_with_cwd(event_loop, geometry, None);
    }

    /// CWD を指定して新しいウィンドウ + セッションを生成する。
    pub(crate) fn create_window_with_cwd(
        &mut self,
        event_loop: &ActiveEventLoop,
        geometry: Option<&WindowGeometry>,
        working_dir: Option<std::path::PathBuf>,
    ) {
        let needs_transparent =
            self.config.window.clamped_opacity() < 1.0 || self.config.window.blur;
        let has_decorations = self.config.window.decorations == Decorations::Full;
        let mut attrs = Window::default_attributes()
            .with_title("SDIT")
            .with_transparent(needs_transparent)
            .with_blur(self.config.window.blur)
            .with_decorations(has_decorations);

        if let Some(geom) = geometry {
            attrs = attrs
                .with_inner_size(winit::dpi::LogicalSize::new(geom.width, geom.height))
                .with_position(winit::dpi::PhysicalPosition::new(geom.x, geom.y));
        } else {
            let metrics = *self.font_ctx.metrics();
            let padding_x = f32::from(self.config.window.clamped_padding_x());
            let padding_y = f32::from(self.config.window.clamped_padding_y());
            let cols = f32::from(self.config.window.clamped_columns());
            let rows = f32::from(self.config.window.clamped_rows());
            let width = f64::from(cols * metrics.cell_width + 2.0 * padding_x);
            let height = f64::from(rows * metrics.cell_height + 2.0 * padding_y);
            attrs = attrs.with_inner_size(winit::dpi::LogicalSize::new(width, height));
            if let Some((x, y)) = self.config.window.clamped_position() {
                attrs = attrs.with_position(winit::dpi::PhysicalPosition::new(x, y));
            } else if let Some(pos) = self.cascade_position() {
                attrs = attrs.with_position(pos);
            }
        }

        let window = match event_loop.create_window(attrs) {
            Ok(w) => {
                w.set_ime_allowed(true);
                #[cfg(target_os = "macos")]
                w.set_option_as_alt(config_option_as_alt_to_winit(self.config.option_as_alt));
                // startup_mode を適用する（geometry 復元時はスキップ）
                if geometry.is_none() {
                    match self.config.window.startup_mode {
                        StartupMode::Maximized => w.set_maximized(true),
                        StartupMode::Fullscreen => {
                            w.set_fullscreen(Some(Fullscreen::Borderless(None)));
                        }
                        StartupMode::Windowed => {}
                    }
                }
                // always_on_top を適用する
                if self.config.window.always_on_top {
                    w.set_window_level(WindowLevel::AlwaysOnTop);
                }
                // resize_increments: セルサイズの整数倍でリサイズするヒントを設定する
                if self.config.window.resize_increments {
                    let m = *self.font_ctx.metrics();
                    if m.cell_width > 0.0
                        && m.cell_height > 0.0
                        && m.cell_width.is_finite()
                        && m.cell_height.is_finite()
                    {
                        w.set_resize_increments(Some(winit::dpi::LogicalSize::new(
                            f64::from(m.cell_width),
                            f64::from(m.cell_height),
                        )));
                    }
                }
                Arc::new(w)
            }
            Err(e) => {
                log::error!("Window creation failed: {e}");
                return;
            }
        };

        let prefer_wide =
            self.config.window.colorspace == sdit_core::config::WindowColorspace::DisplayP3;
        let gpu = match GpuContext::new(&window, prefer_wide) {
            Ok(g) => g,
            Err(e) => {
                log::error!("GPU context creation failed: {e}");
                return;
            }
        };

        let metrics = *self.font_ctx.metrics();
        let padding_x = f32::from(self.config.window.clamped_padding_x());
        let padding_y = f32::from(self.config.window.clamped_padding_y());
        let (cols, rows) = calc_grid_size(
            (gpu.surface_config.width as f32 - 2.0 * padding_x).max(0.0),
            (gpu.surface_config.height as f32 - 2.0 * padding_y).max(0.0),
            metrics.cell_width,
            metrics.cell_height,
        );

        // --- Session 生成 ---
        let Some(session_id) = self.spawn_session_with_cwd(rows, cols, working_dir) else {
            return;
        };
        let session = self.session_mgr.get(session_id).unwrap();

        // --- GPU パイプライン初期化 ---
        let mut atlas = Atlas::new(&gpu.device, 512);
        let cell_size = [metrics.cell_width, metrics.cell_height];
        let surface_size = [gpu.surface_config.width as f32, gpu.surface_config.height as f32];
        let atlas_size_f32 = atlas.size() as f32;

        let state_lock =
            session.term_state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let grid = state_lock.terminal.grid();
        let mut cell_pipeline =
            CellPipeline::new(&gpu.device, gpu.surface_config.format, &atlas, rows * cols);
        let cursor_col = grid.cursor.point.column.0;
        #[allow(clippy::cast_sign_loss)]
        let cursor_row = grid.cursor.point.line.0 as usize;
        cell_pipeline.update_from_grid(
            &gpu.queue,
            grid,
            &mut self.font_ctx,
            &mut atlas,
            atlas_size_f32,
            cell_size,
            surface_size,
            Some((cursor_col, cursor_row)),
            None, // cursor_color: 初期描画では色なし（デフォルト反転）
            None,
            None,
            None,
            None,
            None,  // selection_fg: 初期描画では None
            None,  // selection_bg: 初期描画では None
            1.0,   // minimum_contrast: 初期描画ではデフォルト（無効）
            false, // bold_is_bright: 初期描画ではデフォルト
            0.5,   // faint_opacity: 初期描画ではデフォルト
        );
        drop(state_lock);

        atlas.upload_if_dirty(&gpu.queue);

        // サイドバーパイプライン（初期容量は小さく）
        let sidebar_pipeline =
            CellPipeline::new(&gpu.device, gpu.surface_config.format, &atlas, 100);

        // 背景画像パイプライン（設定されている場合のみ）
        let bg_pipeline = self.config.window.background_image.as_deref().and_then(|path| {
            load_background_image(path).and_then(|(data, w, h)| {
                let fit_mode = match self.config.window.background_image_fit {
                    BackgroundImageFit::Contain => 0,
                    BackgroundImageFit::Cover => 1,
                    BackgroundImageFit::Fill => 2,
                };
                let opacity = self.config.window.clamped_background_image_opacity();
                BackgroundPipeline::new(
                    &gpu.device,
                    &gpu.queue,
                    gpu.surface_config.format,
                    &data,
                    w,
                    h,
                    fit_mode,
                    opacity,
                    surface_size,
                )
            })
        });

        // --- 登録 ---
        let window_id = window.id();
        self.session_to_window.insert(session_id, window_id);
        self.windows.insert(
            window_id,
            WindowState {
                window,
                gpu,
                cell_pipeline,
                sidebar_pipeline,
                bg_pipeline,
                atlas,
                sessions: vec![session_id],
                active_index: 0,
                sidebar: SidebarState::new(),
                visual_bell: VisualBell::new(self.config.bell.clamped_duration_ms()),
            },
        );

        log::info!("Created window {window_id:?} with session {}", session_id.0);

        // 新ウィンドウにフォーカスを移す
        if let Some(ws) = self.windows.get(&window_id) {
            ws.window.focus_window();
        }

        // 初回描画を明示的にトリガーする（add_session_to_window と同様）
        self.redraw_session(session_id);
    }

    /// 既存ウィンドウに新しいセッションを追加する。
    pub(crate) fn add_session_to_window(&mut self, window_id: WindowId) {
        let Some(ws) = self.windows.get(&window_id) else { return };
        let metrics = *self.font_ctx.metrics();
        let sidebar_w = ws.sidebar.width_px(metrics.cell_width);
        let padding_x = f32::from(self.config.window.clamped_padding_x());
        let padding_y = f32::from(self.config.window.clamped_padding_y());
        let term_width =
            (ws.gpu.surface_config.width as f32 - sidebar_w - 2.0 * padding_x).max(0.0);
        let term_height = (ws.gpu.surface_config.height as f32 - 2.0 * padding_y).max(0.0);
        let (cols, rows) =
            calc_grid_size(term_width, term_height, metrics.cell_width, metrics.cell_height);

        // inherit_working_directory: アクティブセッションの CWD を継承する
        let inherit_cwd = if self.config.window.inherit_working_directory {
            let active_sid = ws.active_session_id();
            self.session_mgr.get(active_sid).and_then(|s| s.cwd.clone())
        } else {
            None
        };

        let Some(session_id) = self.spawn_session_with_cwd(rows, cols, inherit_cwd) else {
            return;
        };

        self.session_to_window.insert(session_id, window_id);

        let ws = self.windows.get_mut(&window_id).unwrap();
        ws.sessions.push(session_id);
        ws.active_index = ws.sessions.len() - 1;
        ws.sidebar.auto_update(ws.sessions.len());

        log::info!(
            "Added session {} to window {window_id:?} (total: {})",
            session_id.0,
            ws.sessions.len()
        );

        // 新しいアクティブセッションで再描画
        self.redraw_session(session_id);
    }

    /// 既存ウィンドウに CWD を指定して新しいセッションを追加する。
    ///
    /// セッション復元時に使用する。`cwd` が `None` の場合は通常の CWD 継承ロジックを使わず
    /// デフォルト（ホームディレクトリ）で起動する。
    pub(crate) fn add_session_to_window_with_cwd(
        &mut self,
        window_id: WindowId,
        cwd: Option<std::path::PathBuf>,
    ) {
        let Some(ws) = self.windows.get(&window_id) else { return };
        let metrics = *self.font_ctx.metrics();
        let sidebar_w = ws.sidebar.width_px(metrics.cell_width);
        let padding_x = f32::from(self.config.window.clamped_padding_x());
        let padding_y = f32::from(self.config.window.clamped_padding_y());
        let term_width =
            (ws.gpu.surface_config.width as f32 - sidebar_w - 2.0 * padding_x).max(0.0);
        let term_height = (ws.gpu.surface_config.height as f32 - 2.0 * padding_y).max(0.0);
        let (cols, rows) =
            calc_grid_size(term_width, term_height, metrics.cell_width, metrics.cell_height);

        let Some(session_id) = self.spawn_session_with_cwd(rows, cols, cwd) else {
            return;
        };

        self.session_to_window.insert(session_id, window_id);

        let ws = self.windows.get_mut(&window_id).unwrap();
        ws.sessions.push(session_id);
        // アクティブインデックスは呼び出し側が設定するためここでは変更しない
        ws.sidebar.auto_update(ws.sessions.len());

        log::info!(
            "Added session {} (with cwd) to window {window_id:?} (total: {})",
            session_id.0,
            ws.sessions.len()
        );
    }

    /// アクティブセッションを閉じる。最後の1つならウィンドウごと閉じる。
    pub(crate) fn remove_active_session(&mut self, window_id: WindowId) -> bool {
        let Some(ws) = self.windows.get(&window_id) else {
            return false;
        };

        if ws.sessions.len() <= 1 {
            // 最後のセッション → ウィンドウごと閉じる
            self.close_window(window_id);
            return true;
        }

        let removed_sid = ws.active_session_id();
        let ws = self.windows.get_mut(&window_id).unwrap();
        ws.sessions.remove(ws.active_index);
        if ws.active_index >= ws.sessions.len() {
            ws.active_index = ws.sessions.len() - 1;
        }
        ws.sidebar.auto_update(ws.sessions.len());

        self.session_to_window.remove(&removed_sid);
        self.session_mgr.remove(removed_sid);

        log::info!("Removed session {} from window {window_id:?}", removed_sid.0);

        // 新しいアクティブセッションで再描画
        let new_active = self.windows.get(&window_id).unwrap().active_session_id();
        self.redraw_session(new_active);
        false
    }

    /// アクティブセッションを切り替える（+1 で次、-1 で前）。
    #[allow(clippy::cast_possible_wrap)]
    pub(crate) fn switch_session(&mut self, window_id: WindowId, direction: i32) {
        let Some(ws) = self.windows.get_mut(&window_id) else { return };
        if ws.sessions.len() <= 1 {
            return;
        }

        let len = ws.sessions.len() as i32;
        let new_index = ((ws.active_index as i32 + direction) % len + len) % len;
        ws.active_index = new_index as usize;

        let sid = ws.active_session_id();
        log::info!("Switched to session {} in window {window_id:?}", sid.0);

        self.redraw_session(sid);
    }

    /// 指定ウィンドウとそのセッション群を閉じる。
    pub(crate) fn close_window(&mut self, window_id: WindowId) {
        if let Some(ws) = self.windows.remove(&window_id) {
            for &sid in &ws.sessions {
                self.session_to_window.remove(&sid);
                self.session_mgr.remove(sid);
            }
            log::info!(
                "Closed window {window_id:?}, sessions {:?}",
                ws.sessions.iter().map(|s| s.0).collect::<Vec<_>>()
            );
        }

        // 残存ウィンドウのジオメトリとセッションを保存する
        self.save_session_snapshot();
    }

    /// CursorLeft 時にサイドバードラッグ中ならタブを切り出す（Chrome-like UX）。
    pub(crate) fn drag_detach_on_cursor_left(
        &mut self,
        window_id: winit::window::WindowId,
        event_loop: &winit::event_loop::ActiveEventLoop,
    ) {
        if self.drag_source_row.is_none() {
            return;
        }
        self.drag_source_row = None;
        let screen_pos = self
            .windows
            .get(&window_id)
            .and_then(|ws| ws.window.outer_position().ok())
            .zip(self.cursor_position)
            .map(|(win_pos, (cx, cy))| {
                winit::dpi::PhysicalPosition::new(win_pos.x + cx as i32, win_pos.y + cy as i32)
            });
        self.detach_session_to_new_window(window_id, event_loop, screen_pos);
    }

    /// アクティブセッションを新しいウィンドウに切り出す（PTY は維持）。
    ///
    /// セッションが1つしかない場合は何もしない（切出す意味がない）。
    /// `cursor_pos`: ドラッグ切り出し時に新ウィンドウを配置する画面座標。None のときはカスケード配置。
    pub(crate) fn detach_session_to_new_window(
        &mut self,
        source_window_id: WindowId,
        event_loop: &ActiveEventLoop,
        cursor_pos: Option<winit::dpi::PhysicalPosition<i32>>,
    ) {
        let Some(ws) = self.windows.get(&source_window_id) else { return };
        if ws.sessions.len() <= 1 {
            return; // 最後の1つは切り出せない
        }

        let detach_sid = ws.active_session_id();
        let original_index = ws.active_index;

        // 元ウィンドウからセッションを除去
        let ws = self.windows.get_mut(&source_window_id).unwrap();
        ws.sessions.remove(ws.active_index);
        if ws.active_index >= ws.sessions.len() {
            ws.active_index = ws.sessions.len().saturating_sub(1);
        }
        ws.sidebar.auto_update(ws.sessions.len());

        // ロールバック用のクロージャ的マクロ（元の位置に復元）
        macro_rules! rollback {
            () => {{
                let ws = self.windows.get_mut(&source_window_id).unwrap();
                ws.sessions.insert(original_index, detach_sid);
                ws.active_index = original_index;
                ws.sidebar.auto_update(ws.sessions.len());
            }};
        }

        // 新しいウィンドウを作成（元ウィンドウからカスケード配置）
        let needs_transparent =
            self.config.window.clamped_opacity() < 1.0 || self.config.window.blur;
        let detach_metrics = *self.font_ctx.metrics();
        let detach_padding_x = f32::from(self.config.window.clamped_padding_x());
        let detach_padding_y = f32::from(self.config.window.clamped_padding_y());
        let detach_cols = f32::from(self.config.window.clamped_columns());
        let detach_rows = f32::from(self.config.window.clamped_rows());
        let detach_width =
            f64::from(detach_cols * detach_metrics.cell_width + 2.0 * detach_padding_x);
        let detach_height =
            f64::from(detach_rows * detach_metrics.cell_height + 2.0 * detach_padding_y);
        let mut attrs = Window::default_attributes()
            .with_title("SDIT")
            .with_inner_size(winit::dpi::LogicalSize::new(detach_width, detach_height))
            .with_transparent(needs_transparent)
            .with_blur(self.config.window.blur);

        // ドラッグ切り出しの場合はカーソル座標、それ以外はカスケード配置
        let placement = cursor_pos.or_else(|| self.cascade_position());
        if let Some(pos) = placement {
            attrs = attrs.with_position(pos);
        }

        let new_window = match event_loop.create_window(attrs) {
            Ok(w) => {
                w.set_ime_allowed(true);
                #[cfg(target_os = "macos")]
                w.set_option_as_alt(config_option_as_alt_to_winit(self.config.option_as_alt));
                if self.config.window.resize_increments {
                    let m = *self.font_ctx.metrics();
                    if m.cell_width > 0.0
                        && m.cell_height > 0.0
                        && m.cell_width.is_finite()
                        && m.cell_height.is_finite()
                    {
                        w.set_resize_increments(Some(winit::dpi::LogicalSize::new(
                            f64::from(m.cell_width),
                            f64::from(m.cell_height),
                        )));
                    }
                }
                Arc::new(w)
            }
            Err(e) => {
                log::error!("Window creation failed for detach: {e}");
                rollback!();
                return;
            }
        };

        let prefer_wide =
            self.config.window.colorspace == sdit_core::config::WindowColorspace::DisplayP3;
        let gpu = match GpuContext::new(&new_window, prefer_wide) {
            Ok(g) => g,
            Err(e) => {
                log::error!("GPU context creation failed for detach: {e}");
                rollback!();
                return;
            }
        };

        let metrics = *self.font_ctx.metrics();
        let detach_surface_size =
            [gpu.surface_config.width as f32, gpu.surface_config.height as f32];
        let atlas = Atlas::new(&gpu.device, 512);
        let cell_pipeline =
            CellPipeline::new(&gpu.device, gpu.surface_config.format, &atlas, 80 * 24);
        let sidebar_pipeline =
            CellPipeline::new(&gpu.device, gpu.surface_config.format, &atlas, 100);

        // 背景画像パイプライン（設定されている場合のみ）
        let detach_bg_pipeline = self.config.window.background_image.as_deref().and_then(|path| {
            load_background_image(path).and_then(|(data, w, h)| {
                let fit_mode = match self.config.window.background_image_fit {
                    BackgroundImageFit::Contain => 0,
                    BackgroundImageFit::Cover => 1,
                    BackgroundImageFit::Fill => 2,
                };
                let opacity = self.config.window.clamped_background_image_opacity();
                BackgroundPipeline::new(
                    &gpu.device,
                    &gpu.queue,
                    gpu.surface_config.format,
                    &data,
                    w,
                    h,
                    fit_mode,
                    opacity,
                    detach_surface_size,
                )
            })
        });

        let new_window_id = new_window.id();

        // 新ウィンドウにセッションを登録
        self.session_to_window.insert(detach_sid, new_window_id);
        self.windows.insert(
            new_window_id,
            WindowState {
                window: new_window,
                gpu,
                cell_pipeline,
                sidebar_pipeline,
                bg_pipeline: detach_bg_pipeline,
                atlas,
                sessions: vec![detach_sid],
                active_index: 0,
                sidebar: SidebarState::new(),
                visual_bell: VisualBell::new(self.config.bell.clamped_duration_ms()),
            },
        );

        // 新ウィンドウのサイズに合わせて Terminal + PTY をリサイズ
        let new_ws = self.windows.get(&new_window_id).unwrap();
        let (cols, rows) = calc_grid_size(
            (new_ws.gpu.surface_config.width as f32 - 2.0 * detach_padding_x).max(0.0),
            (new_ws.gpu.surface_config.height as f32 - 2.0 * detach_padding_y).max(0.0),
            metrics.cell_width,
            metrics.cell_height,
        );
        if let Some(session) = self.session_mgr.get(detach_sid) {
            let mut state =
                session.term_state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            state.terminal.resize(rows, cols);
            drop(state);
            let pty_size =
                PtySize::new(rows.try_into().unwrap_or(24), cols.try_into().unwrap_or(80));
            session.resize_pty(pty_size);
        }

        log::info!(
            "Detached session {} from {source_window_id:?} to new window {new_window_id:?}",
            detach_sid.0
        );

        // 両ウィンドウを再描画
        let source_active = self.windows.get(&source_window_id).unwrap().active_session_id();
        self.redraw_session(source_active);
        self.redraw_session(detach_sid);
    }

    /// Quick Terminal をトグルする（macOS のみ）。
    ///
    /// ウィンドウが未生成の場合は新規作成してスライドイン。
    /// 表示中の場合はスライドアウト（非表示）、非表示の場合はスライドイン（表示）。
    #[cfg(target_os = "macos")]
    pub(crate) fn handle_quick_terminal_toggle(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
    ) {
        use crate::quick_terminal::calc_quick_terminal_geometry;

        let Some(qt) = &self.quick_terminal_state else { return };
        let _ = qt; // borrow check のため一旦解放

        let position = self.config.quick_terminal.position;
        let size_ratio = self.config.quick_terminal.clamped_size();
        let anim_duration = self.config.quick_terminal.clamped_animation_duration();

        // 画面サイズを取得（最初のモニターを使用）
        let screen_size = match event_loop.available_monitors().next() {
            Some(m) => {
                let s = m.size();
                if s.width == 0 || s.height == 0 {
                    log::warn!(
                        "handle_quick_terminal_toggle: monitor size is zero ({}x{}), \
                         using fallback (1920, 1080)",
                        s.width,
                        s.height
                    );
                    (1920u32, 1080u32)
                } else {
                    (s.width, s.height)
                }
            }
            None => {
                log::warn!(
                    "handle_quick_terminal_toggle: no monitor found, using fallback (1920, 1080)"
                );
                (1920u32, 1080u32)
            }
        };

        // ウィンドウが未生成の場合は作成する
        let qt = self.quick_terminal_state.as_mut().unwrap();
        if !qt.window_created {
            // t=1.0 の最終位置でウィンドウを作成（画面外に配置してからアニメーション）
            let (x, y, w, h) = calc_quick_terminal_geometry(position, size_ratio, screen_size, 0.0);
            let window_id = self.create_quick_terminal_window(event_loop, x, y, w, h);
            let qt = self.quick_terminal_state.as_mut().unwrap();
            qt.window_id = window_id;
            qt.window_created = window_id.is_some();
        }

        let qt = self.quick_terminal_state.as_mut().unwrap();

        if qt.visible {
            // スライドアウト開始
            qt.start_slide_out(anim_duration);
            log::info!("Quick Terminal: starting slide-out");
        } else {
            // スライドイン開始
            qt.start_slide_in(anim_duration);
            // ウィンドウを画面外の初期位置に移動してから表示
            if let Some(wid) = qt.window_id {
                if let Some(ws) = self.windows.get(&wid) {
                    let (x, y, _, _) =
                        calc_quick_terminal_geometry(position, size_ratio, screen_size, 0.0);
                    let _ = ws.window.set_outer_position(winit::dpi::PhysicalPosition::new(x, y));
                    ws.window.set_visible(true);
                    ws.window.request_redraw();
                }
            }
            log::info!("Quick Terminal: starting slide-in");
        }

        // アニメーション駆動: ウィンドウに再描画を要求してフレームループで位置更新
        if let Some(wid) = self.quick_terminal_state.as_ref().unwrap().window_id {
            if let Some(ws) = self.windows.get(&wid) {
                ws.window.request_redraw();
            }
        }
    }

    /// Quick Terminal のアニメーションフレームを処理する（macOS のみ）。
    ///
    /// `about_to_wait` から呼ばれ、アニメーション中はウィンドウ位置を更新し再描画を要求する。
    #[cfg(target_os = "macos")]
    pub(crate) fn tick_quick_terminal_animation(&mut self) {
        use crate::quick_terminal::{AnimationDirection, calc_quick_terminal_geometry};

        let Some(qt) = &self.quick_terminal_state else { return };
        let Some(wid) = qt.window_id else { return };

        if !qt.is_animating() {
            // アニメーション完了後の後処理（スライドアウト完了 → ウィンドウを非表示）
            // finish_animation() が SlideOut 完了時に visible = false をセットするため、
            // ここではウィンドウの set_visible(false) と finish_animation() のみ呼び出す
            let is_slide_out_complete = self
                .quick_terminal_state
                .as_ref()
                .and_then(|qt| qt.animation.as_ref())
                .is_some_and(|a| a.is_complete() && a.direction == AnimationDirection::SlideOut);
            if is_slide_out_complete {
                if let Some(ws) = self.windows.get(&wid) {
                    ws.window.set_visible(false);
                }
            }
            if let Some(qt) = self.quick_terminal_state.as_mut() {
                qt.finish_animation();
            }
            return;
        }

        let position = self.config.quick_terminal.position;
        let size_ratio = self.config.quick_terminal.clamped_size();

        // 画面サイズ（簡略化: 現在のウィンドウサイズから逆算、実際はモニターサイズを使う）
        // ここでは Quick Terminal ウィンドウ自体のサイズから position に基づいてサイズを計算
        let screen_size = {
            // アニメーション tick ではイベントループが不要なため保存された値を使用
            // Quick Terminal ウィンドウのサイズから逆算する（position と size_ratio に基づく）
            if let Some(ws) = self.windows.get(&wid) {
                let sz = ws.window.inner_size();
                match position {
                    sdit_core::config::QuickTerminalPosition::Top
                    | sdit_core::config::QuickTerminalPosition::Bottom => {
                        // win_w = screen_w, win_h = screen_h * size_ratio
                        let screen_w = sz.width;
                        let screen_h = if size_ratio > 0.0 {
                            (sz.height as f32 / size_ratio) as u32
                        } else {
                            sz.height
                        };
                        (screen_w, screen_h)
                    }
                    sdit_core::config::QuickTerminalPosition::Left
                    | sdit_core::config::QuickTerminalPosition::Right => {
                        let screen_w = if size_ratio > 0.0 {
                            (sz.width as f32 / size_ratio) as u32
                        } else {
                            sz.width
                        };
                        let screen_h = sz.height;
                        (screen_w, screen_h)
                    }
                }
            } else {
                (1920, 1080)
            }
        };

        let qt = self.quick_terminal_state.as_ref().unwrap();
        let anim = qt.animation.as_ref().unwrap();
        let raw_t = anim.progress();
        let t = match anim.direction {
            AnimationDirection::SlideIn => {
                crate::quick_terminal::QuickTerminalAnimation::ease_in_out(raw_t)
            }
            AnimationDirection::SlideOut => {
                crate::quick_terminal::QuickTerminalAnimation::ease_in_out(1.0 - raw_t)
            }
        };

        let (x, y, _, _) = calc_quick_terminal_geometry(position, size_ratio, screen_size, t);
        if let Some(ws) = self.windows.get(&wid) {
            let _ = ws.window.set_outer_position(winit::dpi::PhysicalPosition::new(x, y));
            ws.window.request_redraw();
        }

        // アニメーション完了チェック
        let is_complete = self
            .quick_terminal_state
            .as_ref()
            .and_then(|qt| qt.animation.as_ref())
            .is_some_and(|a| a.is_complete());
        if is_complete {
            let is_slide_out = self
                .quick_terminal_state
                .as_ref()
                .and_then(|qt| qt.animation.as_ref())
                .is_some_and(|a| a.direction == AnimationDirection::SlideOut);
            if is_slide_out {
                if let Some(ws) = self.windows.get(&wid) {
                    ws.window.set_visible(false);
                }
            }
            if let Some(qt) = self.quick_terminal_state.as_mut() {
                qt.finish_animation();
            }
        }
    }

    /// 現在のアプリケーション全体状態をスナップショットとして保存する。
    ///
    /// `config.window.restore_session` が `false` の場合は何もしない。
    pub(crate) fn save_session_snapshot(&self) {
        if !self.config.window.restore_session {
            return;
        }
        let snapshot = AppSnapshot {
            sessions: Vec::new(),
            windows: self.collect_window_geometries(),
            window_sessions: self.collect_window_sessions(),
        };
        if let Err(e) = snapshot.save(&AppSnapshot::default_path()) {
            log::warn!("Failed to save session snapshot: {e}");
        }
        log::info!("Session snapshot saved ({} window(s))", snapshot.windows.len());
    }
}
