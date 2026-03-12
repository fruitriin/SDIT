use std::collections::HashMap;
use std::io::Read;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{Window, WindowId};

use sdit_core::grid::Dimensions;
use sdit_core::pty::{Pty, PtyConfig, PtySize};
use sdit_core::terminal::{TermMode, Terminal};
use sdit_render::atlas::Atlas;
use sdit_render::font::FontContext;
use sdit_render::pipeline::{CellPipeline, CellVertex, GpuContext};
use sdit_session::{Session, SessionId, SessionManager, SidebarState, SpawnParams, TerminalState};

// ---------------------------------------------------------------------------
// カスタムイベント型
// ---------------------------------------------------------------------------

/// winit ユーザーイベント。
#[derive(Debug)]
pub enum SditEvent {
    /// PTY から新しいデータが来た → 対象セッションのウィンドウを再描画。
    PtyOutput(SessionId),
    /// 子プロセスが終了した → 対応ウィンドウを閉じる。
    ChildExit(SessionId, i32),
}

// ---------------------------------------------------------------------------
// WindowState — ウィンドウ1枚分の状態
// ---------------------------------------------------------------------------

/// ウィンドウ1枚が保持する描画コンテキストとセッション参照。
struct WindowState {
    window: Arc<Window>,
    gpu: GpuContext<'static>,
    cell_pipeline: CellPipeline,
    /// サイドバー描画用パイプライン（表示中のみ使用）。
    sidebar_pipeline: CellPipeline,
    atlas: Atlas,
    /// このウィンドウに属するセッション群（タブ順序）。
    sessions: Vec<SessionId>,
    /// アクティブセッションのインデックス（`sessions` 内）。
    active_index: usize,
    /// サイドバー状態。
    sidebar: SidebarState,
}

impl WindowState {
    /// アクティブセッションの `SessionId` を返す。
    ///
    /// # Panics
    ///
    /// `sessions` が空、または `active_index` が範囲外の場合にパニックする。
    /// 設計上 `sessions` は常に1つ以上のエントリを持つ不変条件が保証されている。
    fn active_session_id(&self) -> SessionId {
        debug_assert!(
            self.active_index < self.sessions.len(),
            "active_index ({}) out of bounds (sessions.len() = {})",
            self.active_index,
            self.sessions.len(),
        );
        self.sessions[self.active_index]
    }
}

// ---------------------------------------------------------------------------
// SditApp
// ---------------------------------------------------------------------------

struct SditApp {
    /// `WindowId` → ウィンドウ状態のマッピング。
    windows: HashMap<WindowId, WindowState>,
    /// `SessionId` → `WindowId` の逆引き（`PtyOutput` から正しいウィンドウを特定）。
    session_to_window: HashMap<SessionId, WindowId>,
    /// セッションマネージャ（全セッションを管理）。
    session_mgr: SessionManager,
    /// フォントコンテキスト（全ウィンドウで共有）。
    font_ctx: FontContext,
    /// 解決済みカラーテーブル。
    colors: sdit_config::color::ResolvedColors,
    /// winit modifier キーの状態。
    modifiers: ModifiersState,
    /// winit イベントループへのプロキシ。
    event_proxy: winit::event_loop::EventLoopProxy<SditEvent>,
    /// `SDIT_SMOKE_TEST=1` のとき true。1フレーム描画後に `event_loop.exit()` を呼ぶ。
    smoke_test: bool,
    /// 初回 resumed で最初のウィンドウを作成済みか。
    initialized: bool,
    /// マウスカーソルの現在位置（物理ピクセル）。
    cursor_position: Option<(f64, f64)>,
    /// サイドバー内ドラッグの開始行インデックス。
    drag_source_row: Option<usize>,
    /// テキスト選択の開始点 (col, row)。
    selection_start: Option<(usize, usize)>,
    /// テキスト選択の終了点 (col, row)。ドラッグ中に更新される。
    selection_end: Option<(usize, usize)>,
    /// ターミナル領域でマウスドラッグ中かどうか。
    is_selecting: bool,
}

impl SditApp {
    fn new(
        event_proxy: winit::event_loop::EventLoopProxy<SditEvent>,
        smoke_test: bool,
        config: &sdit_config::Config,
    ) -> Self {
        Self {
            windows: HashMap::new(),
            session_to_window: HashMap::new(),
            session_mgr: SessionManager::new(),
            font_ctx: FontContext::from_config(&config.font),
            colors: sdit_config::color::ResolvedColors::from_theme(&config.colors.theme),
            modifiers: ModifiersState::empty(),
            event_proxy,
            smoke_test,
            initialized: false,
            cursor_position: None,
            drag_source_row: None,
            selection_start: None,
            selection_end: None,
            is_selecting: false,
        }
    }

    /// 新しいセッションを生成して `SessionManager` に登録する。
    ///
    /// GPU パイプラインの初期化は行わない（呼び出し側で描画を更新する）。
    fn spawn_session(&mut self, rows: usize, cols: usize) -> Option<SessionId> {
        let session_id = self.session_mgr.next_id();
        let pty_size = PtySize::new(rows.try_into().unwrap_or(24), cols.try_into().unwrap_or(80));
        let mut pty_config = PtyConfig::default();
        pty_config.env.insert("TERM".to_owned(), "xterm-256color".to_owned());
        pty_config.env.insert("TERM_PROGRAM".to_owned(), "sdit".to_owned());

        let event_proxy = self.event_proxy.clone();
        let sid = session_id;

        let session = match Session::spawn(
            session_id,
            SpawnParams {
                pty_config,
                pty_size,
                terminal_rows: rows,
                terminal_cols: cols,
                scrollback: 10_000,
                spawn_reader:
                    move |pty: Pty,
                          term_state: Arc<Mutex<TerminalState>>,
                          child_exited: Arc<std::sync::atomic::AtomicBool>| {
                        let pty_writer = pty.try_clone_writer().expect("PTY writer clone failed");
                        let (pty_write_tx, pty_write_rx) = mpsc::sync_channel::<Vec<u8>>(64);
                        let reader_proxy = event_proxy.clone();
                        let writer_proxy = event_proxy;

                        let reader =
                            spawn_pty_reader(pty, term_state, reader_proxy, sid, child_exited);
                        let writer = spawn_pty_writer(pty_writer, pty_write_rx, writer_proxy, sid);

                        (reader, writer, pty_write_tx)
                    },
            },
        ) {
            Ok(s) => s,
            Err(e) => {
                log::error!("Session spawn failed: {e}");
                return None;
            }
        };

        self.session_mgr.insert(session);
        Some(session_id)
    }

    /// 既存ウィンドウの位置からカスケード配置のオフセットを計算する。
    ///
    /// 既存ウィンドウがあれば、最後にアクティブだったウィンドウの位置から
    /// (30, 30) ピクセルずらした位置を返す。
    fn cascade_position(&self) -> Option<winit::dpi::PhysicalPosition<i32>> {
        const CASCADE_OFFSET: i32 = 30;
        // 既存ウィンドウから位置を取得（最初に見つかったものを使用）
        for ws in self.windows.values() {
            if let Ok(pos) = ws.window.outer_position() {
                return Some(winit::dpi::PhysicalPosition::new(
                    pos.x + CASCADE_OFFSET,
                    pos.y + CASCADE_OFFSET,
                ));
            }
        }
        None
    }

    /// 新しいウィンドウ + セッションを生成する。
    fn create_window(&mut self, event_loop: &ActiveEventLoop) {
        let mut attrs = Window::default_attributes()
            .with_title("SDIT")
            .with_inner_size(winit::dpi::LogicalSize::new(800.0_f64, 600.0_f64));

        if let Some(pos) = self.cascade_position() {
            attrs = attrs.with_position(pos);
        }

        let window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
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
            },
        );

        log::info!("Created window {window_id:?} with session {}", session_id.0);

        // 初回描画を明示的にトリガーする（add_session_to_window と同様）
        self.redraw_session(session_id);
    }

    /// 既存ウィンドウに新しいセッションを追加する。
    fn add_session_to_window(&mut self, window_id: WindowId) {
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
    fn remove_active_session(&mut self, window_id: WindowId) -> bool {
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
    fn switch_session(&mut self, window_id: WindowId, direction: i32) {
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
    fn close_window(&mut self, window_id: WindowId) {
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
    }

    /// アクティブセッションを新しいウィンドウに切り出す（PTY は維持）。
    ///
    /// セッションが1つしかない場合は何もしない（切出す意味がない）。
    fn detach_session_to_new_window(
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
        let mut attrs = Window::default_attributes()
            .with_title("SDIT")
            .with_inner_size(winit::dpi::LogicalSize::new(800.0_f64, 600.0_f64));

        if let Some(pos) = self.cascade_position() {
            attrs = attrs.with_position(pos);
        }

        let new_window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
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

    /// PTY 出力があったときに Terminal の Grid から GPU バッファを更新する。
    ///
    /// 非アクティブセッションの出力は描画をスキップする（Terminal には蓄積される）。
    fn redraw_session(&mut self, session_id: SessionId) {
        let Some(&window_id) = self.session_to_window.get(&session_id) else { return };
        let Some(ws) = self.windows.get_mut(&window_id) else { return };

        // 非アクティブセッションの出力は描画しない
        if ws.active_session_id() != session_id {
            return;
        }

        let Some(session) = self.session_mgr.get(session_id) else { return };

        let metrics = *self.font_ctx.metrics();
        let cell_size = [metrics.cell_width, metrics.cell_height];
        let surface_size =
            [ws.gpu.surface_config.width as f32, ws.gpu.surface_config.height as f32];
        let atlas_size_f32 = ws.atlas.size() as f32;

        // サイドバー表示中はターミナル描画を右にオフセット
        let sidebar_width_px = ws.sidebar.width_px(metrics.cell_width);

        let state_lock =
            session.term_state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let grid = state_lock.terminal.grid();

        let grid_rows = grid.screen_lines();
        let grid_cols = grid.columns();
        let needed = grid_rows * grid_cols;
        ws.cell_pipeline.ensure_capacity(&ws.gpu.device, needed);

        // カーソル位置を取得
        let cursor_col = grid.cursor.point.column.0;
        #[allow(clippy::cast_sign_loss)]
        let cursor_row = grid.cursor.point.line.0 as usize;
        let cursor_pos = Some((cursor_col, cursor_row));

        let selection = match (self.selection_start, self.selection_end) {
            (Some(s), Some(e)) => Some((s, e)),
            _ => None,
        };
        ws.cell_pipeline.update_from_grid(
            &ws.gpu.queue,
            grid,
            &mut self.font_ctx,
            &mut ws.atlas,
            atlas_size_f32,
            cell_size,
            surface_size,
            cursor_pos,
            selection,
        );
        drop(state_lock);

        // ターミナルパイプラインの origin_x を設定
        let rows_f32 = grid_rows as f32;
        let cols_f32 = grid_cols as f32;
        ws.cell_pipeline.update_uniforms(
            &ws.gpu.queue,
            cell_size,
            [cols_f32, rows_f32],
            surface_size,
            atlas_size_f32,
            sidebar_width_px,
        );

        // サイドバー描画
        if ws.sidebar.visible {
            let sidebar_cells = build_sidebar_cells(
                &ws.sessions,
                ws.active_index,
                &ws.sidebar,
                &metrics,
                surface_size,
                &mut self.font_ctx,
                &mut ws.atlas,
                &self.colors,
            );
            let sidebar_rows = (surface_size[1] / metrics.cell_height).floor().max(1.0) as usize;
            ws.sidebar_pipeline.ensure_capacity(&ws.gpu.device, sidebar_cells.len());
            ws.sidebar_pipeline.update_cells(&ws.gpu.queue, &sidebar_cells);
            ws.sidebar_pipeline.update_uniforms(
                &ws.gpu.queue,
                cell_size,
                [ws.sidebar.width_cells as f32, sidebar_rows as f32],
                surface_size,
                atlas_size_f32,
                0.0, // サイドバー自体は origin_x = 0
            );
        }

        ws.atlas.upload_if_dirty(&ws.gpu.queue);
        ws.window.request_redraw();
    }

    /// ウィンドウリサイズ時に GPU・Terminal を更新する。
    ///
    /// 全セッションの Terminal と PTY をリサイズする。
    fn handle_resize(&mut self, window_id: WindowId, width: u32, height: u32) {
        let Some(ws) = self.windows.get_mut(&window_id) else { return };
        ws.gpu.resize(width, height);

        let metrics = *self.font_ctx.metrics();
        let sidebar_w = ws.sidebar.width_px(metrics.cell_width);
        let term_width = (width as f32 - sidebar_w).max(0.0);
        let (cols, rows) =
            calc_grid_size(term_width, height as f32, metrics.cell_width, metrics.cell_height);

        let session_ids: Vec<SessionId> = ws.sessions.clone();
        for sid in session_ids {
            if let Some(session) = self.session_mgr.get(sid) {
                let mut state =
                    session.term_state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                state.terminal.resize(rows, cols);
                drop(state);

                let pty_size =
                    PtySize::new(rows.try_into().unwrap_or(24), cols.try_into().unwrap_or(80));
                session.resize_pty(pty_size);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ApplicationHandler
// ---------------------------------------------------------------------------

impl ApplicationHandler<SditEvent> for SditApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.initialized {
            return;
        }
        self.initialized = true;
        self.create_window(event_loop);
    }

    #[allow(clippy::too_many_lines)]
    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                self.close_window(id);
                if self.windows.is_empty() {
                    event_loop.exit();
                }
            }

            WindowEvent::Resized(size) => {
                self.handle_resize(id, size.width, size.height);
                // リサイズ後にアクティブセッションを再描画
                if let Some(ws) = self.windows.get(&id) {
                    let sid = ws.active_session_id();
                    self.redraw_session(sid);
                }
            }

            WindowEvent::ModifiersChanged(mods) => {
                self.modifiers = mods.state();
            }

            WindowEvent::KeyboardInput { event: key_event, .. } => {
                if key_event.state == ElementState::Pressed {
                    // Cmd+Shift+N (macOS) でアクティブセッションを新ウィンドウに切出し
                    if is_detach_session_shortcut(&key_event.logical_key, self.modifiers) {
                        self.detach_session_to_new_window(id, event_loop);
                        return;
                    }

                    // Cmd+N (macOS) / Ctrl+Shift+N で新規ウィンドウ
                    if is_new_window_shortcut(&key_event.logical_key, self.modifiers) {
                        self.create_window(event_loop);
                        return;
                    }

                    // Cmd+\ (macOS) / Ctrl+\ でサイドバートグル
                    if is_sidebar_toggle_shortcut(&key_event.logical_key, self.modifiers) {
                        if let Some(ws) = self.windows.get_mut(&id) {
                            ws.sidebar.toggle();
                            let sid = ws.active_session_id();
                            self.redraw_session(sid);
                        }
                        return;
                    }

                    // Cmd+T (macOS) / Ctrl+Shift+T で同一ウィンドウに新規セッション追加
                    if is_add_session_shortcut(&key_event.logical_key, self.modifiers) {
                        self.add_session_to_window(id);
                        return;
                    }

                    // Cmd+W (macOS) / Ctrl+Shift+W でアクティブセッションを閉じる
                    if is_close_session_shortcut(&key_event.logical_key, self.modifiers) {
                        let window_closed = self.remove_active_session(id);
                        if window_closed && self.windows.is_empty() {
                            event_loop.exit();
                        }
                        return;
                    }

                    // Ctrl+Tab / Cmd+Shift+] で次のセッション
                    // Ctrl+Shift+Tab / Cmd+Shift+[ で前のセッション
                    if let Some(dir) =
                        session_switch_direction(&key_event.logical_key, self.modifiers)
                    {
                        self.switch_session(id, dir);
                        return;
                    }

                    let Some(ws) = self.windows.get(&id) else { return };
                    let Some(session) = self.session_mgr.get(ws.active_session_id()) else {
                        return;
                    };

                    let mode = session
                        .term_state
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .terminal
                        .mode();

                    // キー入力時に選択を解除
                    self.selection_start = None;
                    self.selection_end = None;

                    if let Some(bytes) = key_to_bytes(&key_event.logical_key, self.modifiers, mode)
                    {
                        if let Err(e) = session.pty_io.write_tx.try_send(bytes) {
                            log::warn!("PTY write channel full or closed: {e}");
                        }
                    }
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_position = Some((position.x, position.y));

                // ドラッグ中: サイドバー内で行が変わったらタブ順序を入れ替え
                if let Some(drag_row) = self.drag_source_row {
                    if let Some(ws) = self.windows.get_mut(&id) {
                        let metrics = *self.font_ctx.metrics();
                        if let Some(target_row) = ws.sidebar.hit_test(
                            position.y as f32,
                            metrics.cell_height,
                            ws.sessions.len(),
                        ) {
                            if target_row != drag_row {
                                ws.sessions.swap(drag_row, target_row);
                                // active_index を追従させる
                                if ws.active_index == drag_row {
                                    ws.active_index = target_row;
                                } else if ws.active_index == target_row {
                                    ws.active_index = drag_row;
                                }
                                self.drag_source_row = Some(target_row);
                                let sid = ws.active_session_id();
                                self.redraw_session(sid);
                            }
                        }
                    }
                }

                // テキスト選択中: selection_end を更新
                if self.is_selecting {
                    if let Some(ws) = self.windows.get(&id) {
                        let metrics = *self.font_ctx.metrics();
                        let sidebar_w = ws.sidebar.width_px(metrics.cell_width);
                        let term_x = position.x as f32 - sidebar_w;
                        let col = (term_x / metrics.cell_width).floor().max(0.0) as usize;
                        let row =
                            (position.y as f32 / metrics.cell_height).floor().max(0.0) as usize;
                        self.selection_end = Some((col, row));
                        let sid = ws.active_session_id();
                        self.redraw_session(sid);
                    }
                }
            }

            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if let Some((x, y)) = self.cursor_position {
                    if let Some(ws) = self.windows.get(&id) {
                        let metrics = *self.font_ctx.metrics();
                        let sidebar_w = ws.sidebar.width_px(metrics.cell_width);
                        // サイドバー領域内のクリック
                        if ws.sidebar.visible && (x as f32) < sidebar_w {
                            if let Some(row) = ws.sidebar.hit_test(
                                y as f32,
                                metrics.cell_height,
                                ws.sessions.len(),
                            ) {
                                // ドラッグ開始を記録
                                self.drag_source_row = Some(row);
                                // クリックでセッション切替
                                if row != ws.active_index {
                                    let ws = self.windows.get_mut(&id).unwrap();
                                    ws.active_index = row;
                                    let sid = ws.active_session_id();
                                    self.redraw_session(sid);
                                }
                            }
                        } else {
                            // ターミナル領域: テキスト選択開始
                            let term_x = x as f32 - sidebar_w;
                            let col = (term_x / metrics.cell_width).floor().max(0.0) as usize;
                            let row = (y as f32 / metrics.cell_height).floor().max(0.0) as usize;
                            self.selection_start = Some((col, row));
                            self.selection_end = Some((col, row));
                            self.is_selecting = true;
                            let sid = ws.active_session_id();
                            self.redraw_session(sid);
                        }
                    }
                }
            }

            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
                self.drag_source_row = None;
                self.is_selecting = false;
            }

            WindowEvent::RedrawRequested => {
                let Some(ws) = self.windows.get(&id) else { return };
                let pipelines: Vec<&CellPipeline> = if ws.sidebar.visible {
                    vec![&ws.sidebar_pipeline, &ws.cell_pipeline]
                } else {
                    vec![&ws.cell_pipeline]
                };
                match ws.gpu.render_frame(&pipelines, self.colors.background) {
                    Ok(()) => {}
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        if let Some(ws) = self.windows.get_mut(&id) {
                            let (w, h) =
                                (ws.gpu.surface_config.width, ws.gpu.surface_config.height);
                            ws.gpu.resize(w, h);
                        }
                    }
                    Err(e) => log::error!("Render error: {e}"),
                }
                // SDIT_SMOKE_TEST=1: 1フレーム描画完了後に正常終了する。
                if self.smoke_test {
                    log::info!("smoke_test: 1 frame rendered, exiting");
                    event_loop.exit();
                }
            }

            _ => {}
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: SditEvent) {
        match event {
            SditEvent::PtyOutput(session_id) => {
                self.redraw_session(session_id);
            }
            SditEvent::ChildExit(session_id, code) => {
                log::info!("Session {} child exited with code {code}", session_id.0);
                if let Some(&window_id) = self.session_to_window.get(&session_id) {
                    let is_only_session =
                        self.windows.get(&window_id).is_none_or(|ws| ws.sessions.len() <= 1);

                    if is_only_session {
                        self.close_window(window_id);
                    } else {
                        // 複数セッションがある場合、終了したセッションだけ除去
                        // 削除順序: ws.sessions → session_to_window → session_mgr
                        if let Some(ws) = self.windows.get_mut(&window_id) {
                            if let Some(pos) = ws.sessions.iter().position(|&s| s == session_id) {
                                ws.sessions.remove(pos);
                                if ws.active_index >= ws.sessions.len() {
                                    ws.active_index = ws.sessions.len().saturating_sub(1);
                                }
                                ws.sidebar.auto_update(ws.sessions.len());
                            }
                        }
                        self.session_to_window.remove(&session_id);
                        self.session_mgr.remove(session_id);
                        if let Some(ws) = self.windows.get(&window_id) {
                            if !ws.sessions.is_empty() {
                                let new_active = ws.active_session_id();
                                self.redraw_session(new_active);
                            }
                        }
                    }
                }
                if self.windows.is_empty() {
                    event_loop.exit();
                }
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // PtyOutput イベント駆動で描画するため、ここでは何もしない。
    }
}

// ---------------------------------------------------------------------------
// 新規ウィンドウショートカット判定
// ---------------------------------------------------------------------------

/// Cmd+\ (macOS) または Ctrl+\ でのサイドバートグルかどうか。
fn is_sidebar_toggle_shortcut(key: &Key, modifiers: ModifiersState) -> bool {
    let is_backslash = matches!(key, Key::Character(s) if s.as_str() == "\\" || s.as_str() == "|");
    if !is_backslash {
        return false;
    }
    if cfg!(target_os = "macos") && modifiers.super_key() {
        return true;
    }
    modifiers.control_key()
}

/// Cmd+Shift+N (macOS) でのセッション切出しショートカットかどうか。
fn is_detach_session_shortcut(key: &Key, modifiers: ModifiersState) -> bool {
    let is_n = matches!(key, Key::Character(s) if s.as_str() == "n" || s.as_str() == "N");
    if !is_n {
        return false;
    }
    // macOS: Cmd+Shift+N
    if cfg!(target_os = "macos") && modifiers.super_key() && modifiers.shift_key() {
        return true;
    }
    false
}

/// Cmd+N (macOS) または Ctrl+Shift+N でのウィンドウ生成ショートカットかどうか。
fn is_new_window_shortcut(key: &Key, modifiers: ModifiersState) -> bool {
    let is_n = matches!(key, Key::Character(s) if s.as_str() == "n" || s.as_str() == "N");
    if !is_n {
        return false;
    }
    // macOS: Cmd+N
    if cfg!(target_os = "macos") && modifiers.super_key() && !modifiers.shift_key() {
        return true;
    }
    // Other: Ctrl+Shift+N
    if modifiers.control_key() && modifiers.shift_key() {
        return true;
    }
    false
}

/// Cmd+T (macOS) または Ctrl+Shift+T でのセッション追加ショートカットかどうか。
fn is_add_session_shortcut(key: &Key, modifiers: ModifiersState) -> bool {
    let is_t = matches!(key, Key::Character(s) if s.as_str() == "t" || s.as_str() == "T");
    if !is_t {
        return false;
    }
    if cfg!(target_os = "macos") && modifiers.super_key() && !modifiers.shift_key() {
        return true;
    }
    modifiers.control_key() && modifiers.shift_key()
}

/// Cmd+W (macOS) または Ctrl+Shift+W でのセッション閉じショートカットかどうか。
fn is_close_session_shortcut(key: &Key, modifiers: ModifiersState) -> bool {
    let is_w = matches!(key, Key::Character(s) if s.as_str() == "w" || s.as_str() == "W");
    if !is_w {
        return false;
    }
    if cfg!(target_os = "macos") && modifiers.super_key() && !modifiers.shift_key() {
        return true;
    }
    modifiers.control_key() && modifiers.shift_key()
}

/// セッション切替ショートカット。次: +1、前: -1 を返す。
fn session_switch_direction(key: &Key, modifiers: ModifiersState) -> Option<i32> {
    match key {
        // Ctrl+Tab → 次、Ctrl+Shift+Tab → 前
        Key::Named(NamedKey::Tab) if modifiers.control_key() => {
            Some(if modifiers.shift_key() { -1 } else { 1 })
        }
        // Cmd+Shift+] → 次（macOS）
        Key::Character(s) if s.as_str() == "]" || s.as_str() == "}" => {
            if cfg!(target_os = "macos") && modifiers.super_key() && modifiers.shift_key() {
                Some(1)
            } else {
                None
            }
        }
        // Cmd+Shift+[ → 前（macOS）
        Key::Character(s) if s.as_str() == "[" || s.as_str() == "{" => {
            if cfg!(target_os = "macos") && modifiers.super_key() && modifiers.shift_key() {
                Some(-1)
            } else {
                None
            }
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// PTY リーダースレッド
// ---------------------------------------------------------------------------

fn spawn_pty_reader(
    mut pty: Pty,
    term_state: Arc<Mutex<TerminalState>>,
    event_proxy: winit::event_loop::EventLoopProxy<SditEvent>,
    session_id: SessionId,
    child_exited: Arc<std::sync::atomic::AtomicBool>,
) -> JoinHandle<()> {
    std::thread::Builder::new()
        .name(format!("pty-reader-{}", session_id.0))
        .spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match pty.read(&mut buf) {
                    Ok(0) => {
                        child_exited.store(true, std::sync::atomic::Ordering::Release);
                        let _ = event_proxy.send_event(SditEvent::ChildExit(session_id, 0));
                        break;
                    }
                    Ok(n) => {
                        {
                            let mut state = term_state
                                .lock()
                                .unwrap_or_else(std::sync::PoisonError::into_inner);
                            let TerminalState { processor, terminal } = &mut *state;
                            processor.advance(terminal, &buf[..n]);
                        }
                        let _ = event_proxy.send_event(SditEvent::PtyOutput(session_id));
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(1));
                    }
                    Err(e) if e.raw_os_error() == Some(5) => {
                        child_exited.store(true, std::sync::atomic::Ordering::Release);
                        let _ = event_proxy.send_event(SditEvent::ChildExit(session_id, 0));
                        break;
                    }
                    Err(e) => {
                        log::error!("PTY read error (session {}): {e}", session_id.0);
                        child_exited.store(true, std::sync::atomic::Ordering::Release);
                        let _ = event_proxy.send_event(SditEvent::ChildExit(session_id, 1));
                        break;
                    }
                }
            }
        })
        .unwrap()
}

fn spawn_pty_writer(
    mut writer: std::fs::File,
    pty_write_rx: mpsc::Receiver<Vec<u8>>,
    event_proxy: winit::event_loop::EventLoopProxy<SditEvent>,
    session_id: SessionId,
) -> JoinHandle<()> {
    std::thread::Builder::new()
        .name(format!("pty-writer-{}", session_id.0))
        .spawn(move || {
            while let Ok(data) = pty_write_rx.recv() {
                if let Err(e) = std::io::Write::write_all(&mut writer, &data) {
                    log::error!("PTY write error (session {}): {e}", session_id.0);
                    let _ = event_proxy.send_event(SditEvent::ChildExit(session_id, 1));
                    break;
                }
            }
        })
        .unwrap()
}

// ---------------------------------------------------------------------------
// キー入力 → PTY バイト列変換
// ---------------------------------------------------------------------------

fn key_to_bytes(key: &Key, modifiers: ModifiersState, mode: TermMode) -> Option<Vec<u8>> {
    match key {
        Key::Character(s) => {
            let bytes = s.as_bytes();
            if bytes.is_empty() {
                return None;
            }
            if modifiers.control_key() && bytes.len() == 1 {
                let b = bytes[0];
                let ctrl_byte = if b.is_ascii_lowercase() {
                    b - b'a' + 1
                } else if b.is_ascii_uppercase() {
                    b - b'A' + 1
                } else {
                    b
                };
                return Some(vec![ctrl_byte]);
            }
            Some(bytes.to_vec())
        }
        Key::Named(named) => {
            let app_cursor = mode.contains(TermMode::APP_CURSOR);
            let s: &[u8] = match named {
                NamedKey::Enter => b"\r",
                NamedKey::Backspace => b"\x7f",
                NamedKey::Tab => b"\t",
                NamedKey::Escape => b"\x1b",
                NamedKey::ArrowUp => {
                    if app_cursor {
                        b"\x1bOA"
                    } else {
                        b"\x1b[A"
                    }
                }
                NamedKey::ArrowDown => {
                    if app_cursor {
                        b"\x1bOB"
                    } else {
                        b"\x1b[B"
                    }
                }
                NamedKey::ArrowRight => {
                    if app_cursor {
                        b"\x1bOC"
                    } else {
                        b"\x1b[C"
                    }
                }
                NamedKey::ArrowLeft => {
                    if app_cursor {
                        b"\x1bOD"
                    } else {
                        b"\x1b[D"
                    }
                }
                NamedKey::Home => b"\x1b[H",
                NamedKey::End => b"\x1b[F",
                NamedKey::PageUp => b"\x1b[5~",
                NamedKey::PageDown => b"\x1b[6~",
                NamedKey::Insert => b"\x1b[2~",
                NamedKey::Delete => b"\x1b[3~",
                NamedKey::Space => b" ",
                _ => return None,
            };
            Some(s.to_vec())
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// ユーティリティ
// ---------------------------------------------------------------------------

fn calc_grid_size(
    surface_width: f32,
    surface_height: f32,
    cell_width: f32,
    cell_height: f32,
) -> (usize, usize) {
    let cols = if cell_width > 0.0 { (surface_width / cell_width).floor() as usize } else { 80 };
    let rows = if cell_height > 0.0 { (surface_height / cell_height).floor() as usize } else { 24 };
    (cols.max(1), rows.max(1))
}

/// サイドバー用の `CellVertex` 列を生成する。
///
/// 各セッションに1行を割り当て、アクティブセッションをハイライトする。
#[allow(clippy::too_many_arguments)]
fn build_sidebar_cells(
    sessions: &[SessionId],
    active_index: usize,
    sidebar: &SidebarState,
    metrics: &sdit_render::font::CellMetrics,
    surface_size: [f32; 2],
    font_ctx: &mut FontContext,
    atlas: &mut Atlas,
    colors: &sdit_config::color::ResolvedColors,
) -> Vec<CellVertex> {
    let width = sidebar.width_cells;
    let total_rows = (surface_size[1] / metrics.cell_height).floor().max(1.0) as usize;
    let atlas_size = atlas.size() as f32;

    let sidebar_bg = colors.sidebar_bg;
    let active_bg = colors.sidebar_active_bg;
    let fg_color = colors.sidebar_fg;
    let dim_fg = colors.sidebar_dim_fg;

    let mut cells = Vec::with_capacity(total_rows * width);

    for row in 0..total_rows {
        let is_session_row = row < sessions.len();
        let is_active = is_session_row && row == active_index;
        let bg = if is_active { active_bg } else { sidebar_bg };
        let fg = if is_session_row { fg_color } else { dim_fg };

        // セッション名を生成（例: "> Session 0" or "  Session 1"）
        let label = if is_session_row {
            let prefix = if is_active { "> " } else { "  " };
            format!("{prefix}Session {}", sessions[row].0)
        } else {
            String::new()
        };
        let label_chars: Vec<char> = label.chars().collect();

        for col in 0..width {
            let ch = label_chars.get(col).copied().unwrap_or(' ');

            let (uv, glyph_offset, glyph_size) =
                if let Some(entry) = font_ctx.rasterize_glyph(ch, atlas) {
                    let r = entry.region;
                    let uv = [
                        r.x as f32 / atlas_size,
                        r.y as f32 / atlas_size,
                        (r.x + r.width) as f32 / atlas_size,
                        (r.y + r.height) as f32 / atlas_size,
                    ];
                    let offset = [entry.placement_left as f32, entry.placement_top as f32];
                    let size = [r.width as f32, r.height as f32];
                    (uv, offset, size)
                } else {
                    ([0.0_f32; 4], [0.0_f32; 2], [0.0_f32; 2])
                };

            cells.push(CellVertex {
                bg,
                fg,
                grid_pos: [col as f32, row as f32],
                uv,
                glyph_offset,
                glyph_size,
                cell_width_scale: 1.0,
            });
        }
    }

    cells
}

// ---------------------------------------------------------------------------
// ヘッドレスモード
// ---------------------------------------------------------------------------

fn grid_contains(terminal: &Terminal, needle: &str) -> bool {
    use sdit_core::index::{Column, Line, Point};
    let rows = terminal.grid().screen_lines();
    let cols = terminal.grid().columns();
    (0..rows).any(|r| {
        #[allow(clippy::cast_possible_wrap)]
        let line = Line(r as i32);
        let mut row = String::new();
        for c in 0..cols {
            row.push(terminal.grid()[Point::new(line, Column(c))].c);
        }
        row.contains(needle)
    })
}

fn run_headless() -> ! {
    use sdit_core::terminal::Processor;

    let size = PtySize::new(24, 80);
    let config = PtyConfig {
        shell: Some("/bin/sh".to_owned()),
        args: vec!["-c".to_owned(), "echo SDIT_HEADLESS_OK".to_owned()],
        working_directory: None,
        env: std::collections::HashMap::new(),
    };

    let mut pty = match Pty::spawn(&config, size) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("headless: PTY spawn failed: {e}");
            std::process::exit(1);
        }
    };

    let mut terminal = Terminal::new(24, 80, 1000);
    let mut processor = Processor::new();

    let timeout = std::time::Duration::from_secs(5);
    let deadline = std::time::Instant::now() + timeout;
    let mut buf = [0u8; 4096];

    loop {
        if grid_contains(&terminal, "SDIT_HEADLESS_OK") {
            std::process::exit(0);
        }

        if std::time::Instant::now() >= deadline {
            eprintln!("headless: timeout waiting for SDIT_HEADLESS_OK");
            std::process::exit(1);
        }

        match pty.read(&mut buf) {
            Ok(0) => {
                if grid_contains(&terminal, "SDIT_HEADLESS_OK") {
                    std::process::exit(0);
                } else {
                    eprintln!("headless: EOF reached without finding SDIT_HEADLESS_OK");
                    std::process::exit(1);
                }
            }
            Ok(n) => {
                processor.advance(&mut terminal, &buf[..n]);
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            Err(e) if e.raw_os_error() == Some(5) => {
                if grid_contains(&terminal, "SDIT_HEADLESS_OK") {
                    std::process::exit(0);
                } else {
                    eprintln!("headless: EIO without finding SDIT_HEADLESS_OK");
                    std::process::exit(1);
                }
            }
            Err(e) => {
                eprintln!("headless: PTY read error: {e}");
                std::process::exit(1);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// エントリーポイント
// ---------------------------------------------------------------------------

fn main() {
    env_logger::init();

    if std::env::args().any(|a| a == "--headless") {
        log::info!("SDIT starting in headless mode");
        run_headless();
    }

    let smoke_test =
        cfg!(debug_assertions) && std::env::var("SDIT_SMOKE_TEST").as_deref() == Ok("1");

    let config = sdit_config::Config::load(&sdit_config::Config::default_path());
    log::info!("SDIT starting (font: {} {}px)", config.font.family, config.font.size);
    let event_loop = EventLoop::<SditEvent>::with_user_event().build().unwrap();
    let proxy = event_loop.create_proxy();
    let mut app = SditApp::new(proxy, smoke_test, &config);
    event_loop.run_app(&mut app).unwrap();
}
