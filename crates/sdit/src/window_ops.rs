use std::sync::Arc;

use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

#[cfg(target_os = "macos")]
use winit::platform::macos::{OptionAsAlt as WinitOptionAsAlt, WindowExtMacOS};

use sdit_core::pty::PtySize;
use sdit_core::render::atlas::Atlas;
use sdit_core::render::pipeline::{CellPipeline, GpuContext};
use sdit_core::session::{AppSnapshot, SessionSnapshot, SidebarState, WindowGeometry};

use crate::app::{SditApp, VisualBell, WindowState};
use crate::window::calc_grid_size;

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

    /// 現在のセッション群のスナップショットを収集する。
    ///
    /// 現時点では cwd の取得はサポートしていないため、空のリストを返す。
    /// セッション復元は将来のフェーズで実装予定。
    fn collect_session_snapshots() -> Vec<SessionSnapshot> {
        Vec::new()
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
        let needs_transparent =
            self.config.window.clamped_opacity() < 1.0 || self.config.window.blur;
        let mut attrs = Window::default_attributes()
            .with_title("SDIT")
            .with_transparent(needs_transparent)
            .with_blur(self.config.window.blur);

        if let Some(geom) = geometry {
            attrs = attrs
                .with_inner_size(winit::dpi::LogicalSize::new(geom.width, geom.height))
                .with_position(winit::dpi::PhysicalPosition::new(geom.x, geom.y));
        } else {
            attrs = attrs.with_inner_size(winit::dpi::LogicalSize::new(800.0_f64, 600.0_f64));
            if let Some(pos) = self.cascade_position() {
                attrs = attrs.with_position(pos);
            }
        }

        let window = match event_loop.create_window(attrs) {
            Ok(w) => {
                w.set_ime_allowed(true);
                #[cfg(target_os = "macos")]
                w.set_option_as_alt(config_option_as_alt_to_winit(self.config.option_as_alt));
                Arc::new(w)
            }
            Err(e) => {
                log::error!("Window creation failed: {e}");
                return;
            }
        };

        let gpu = match GpuContext::new(&window) {
            Ok(g) => g,
            Err(e) => {
                log::error!("GPU context creation failed: {e}");
                return;
            }
        };

        let metrics = *self.font_ctx.metrics();
        let (cols, rows) = calc_grid_size(
            gpu.surface_config.width as f32,
            gpu.surface_config.height as f32,
            metrics.cell_width,
            metrics.cell_height,
        );

        // --- Session 生成 ---
        let Some(session_id) = self.spawn_session(rows, cols) else {
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
            None,
            None,
            None,
            None,
        );
        drop(state_lock);

        atlas.upload_if_dirty(&gpu.queue);

        // サイドバーパイプライン（初期容量は小さく）
        let sidebar_pipeline =
            CellPipeline::new(&gpu.device, gpu.surface_config.format, &atlas, 100);

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
                atlas,
                sessions: vec![session_id],
                active_index: 0,
                sidebar: SidebarState::new(),
                visual_bell: VisualBell::new(self.config.bell.clamped_duration_ms()),
            },
        );

        log::info!("Created window {window_id:?} with session {}", session_id.0);

        // 初回描画を明示的にトリガーする（add_session_to_window と同様）
        self.redraw_session(session_id);
    }

    /// 既存ウィンドウに新しいセッションを追加する。
    pub(crate) fn add_session_to_window(&mut self, window_id: WindowId) {
        let Some(ws) = self.windows.get(&window_id) else { return };
        let metrics = *self.font_ctx.metrics();
        let sidebar_w = ws.sidebar.width_px(metrics.cell_width);
        let term_width = (ws.gpu.surface_config.width as f32 - sidebar_w).max(0.0);
        let (cols, rows) = calc_grid_size(
            term_width,
            ws.gpu.surface_config.height as f32,
            metrics.cell_width,
            metrics.cell_height,
        );

        let Some(session_id) = self.spawn_session(rows, cols) else {
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
        let snapshot = AppSnapshot {
            sessions: Self::collect_session_snapshots(),
            windows: self.collect_window_geometries(),
        };
        if let Err(e) = snapshot.save(&AppSnapshot::default_path()) {
            log::warn!("Failed to save window geometry: {e}");
        }
    }

    /// アクティブセッションを新しいウィンドウに切り出す（PTY は維持）。
    ///
    /// セッションが1つしかない場合は何もしない（切出す意味がない）。
    pub(crate) fn detach_session_to_new_window(
        &mut self,
        source_window_id: WindowId,
        event_loop: &ActiveEventLoop,
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
        let mut attrs = Window::default_attributes()
            .with_title("SDIT")
            .with_inner_size(winit::dpi::LogicalSize::new(800.0_f64, 600.0_f64))
            .with_transparent(needs_transparent)
            .with_blur(self.config.window.blur);

        if let Some(pos) = self.cascade_position() {
            attrs = attrs.with_position(pos);
        }

        let new_window = match event_loop.create_window(attrs) {
            Ok(w) => {
                w.set_ime_allowed(true);
                #[cfg(target_os = "macos")]
                w.set_option_as_alt(config_option_as_alt_to_winit(self.config.option_as_alt));
                Arc::new(w)
            }
            Err(e) => {
                log::error!("Window creation failed for detach: {e}");
                rollback!();
                return;
            }
        };

        let gpu = match GpuContext::new(&new_window) {
            Ok(g) => g,
            Err(e) => {
                log::error!("GPU context creation failed for detach: {e}");
                rollback!();
                return;
            }
        };

        let metrics = *self.font_ctx.metrics();
        let atlas = Atlas::new(&gpu.device, 512);
        let cell_pipeline =
            CellPipeline::new(&gpu.device, gpu.surface_config.format, &atlas, 80 * 24);
        let sidebar_pipeline =
            CellPipeline::new(&gpu.device, gpu.surface_config.format, &atlas, 100);

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
            new_ws.gpu.surface_config.width as f32,
            new_ws.gpu.surface_config.height as f32,
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
}
