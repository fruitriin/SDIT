use std::collections::HashMap;
use std::io::Read;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{Window, WindowId};

use sdit_core::grid::Dimensions;
use sdit_core::pty::{Pty, PtyConfig, PtySize};
use sdit_core::terminal::{TermMode, Terminal};
use sdit_render::atlas::Atlas;
use sdit_render::font::FontContext;
use sdit_render::pipeline::{CellPipeline, GpuContext};
use sdit_session::{Session, SessionId, SessionManager, SpawnParams, TerminalState};

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
    atlas: Atlas,
    session_id: SessionId,
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
    /// winit modifier キーの状態。
    modifiers: ModifiersState,
    /// winit イベントループへのプロキシ。
    event_proxy: winit::event_loop::EventLoopProxy<SditEvent>,
    /// `SDIT_SMOKE_TEST=1` のとき true。1フレーム描画後に `event_loop.exit()` を呼ぶ。
    smoke_test: bool,
    /// 初回 resumed で最初のウィンドウを作成済みか。
    initialized: bool,
}

impl SditApp {
    fn new(event_proxy: winit::event_loop::EventLoopProxy<SditEvent>, smoke_test: bool) -> Self {
        Self {
            windows: HashMap::new(),
            session_to_window: HashMap::new(),
            session_mgr: SessionManager::new(),
            font_ctx: FontContext::new(14.0, 1.2),
            modifiers: ModifiersState::empty(),
            event_proxy,
            smoke_test,
            initialized: false,
        }
    }

    /// 新しいウィンドウ + セッションを生成する。
    fn create_window(&mut self, event_loop: &ActiveEventLoop) {
        let attrs = Window::default_attributes()
            .with_title("SDIT")
            .with_inner_size(winit::dpi::LogicalSize::new(800.0_f64, 600.0_f64));

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
        let session_id = self.session_mgr.next_id();
        let pty_size = PtySize::new(rows.try_into().unwrap_or(24), cols.try_into().unwrap_or(80));
        let mut pty_config = PtyConfig::default();
        pty_config.env.insert("TERM".to_owned(), "xterm-256color".to_owned());

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
                return;
            }
        };

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
        cell_pipeline.update_from_grid(
            &gpu.queue,
            grid,
            &mut self.font_ctx,
            &mut atlas,
            atlas_size_f32,
            cell_size,
            surface_size,
        );
        drop(state_lock);

        atlas.upload_if_dirty(&gpu.queue);

        // --- 登録 ---
        let window_id = window.id();
        self.session_to_window.insert(session_id, window_id);
        self.session_mgr.insert(session);
        self.windows
            .insert(window_id, WindowState { window, gpu, cell_pipeline, atlas, session_id });

        log::info!("Created window {window_id:?} with session {}", session_id.0);
    }

    /// 指定ウィンドウとそのセッションを閉じる。
    fn close_window(&mut self, window_id: WindowId) {
        if let Some(ws) = self.windows.remove(&window_id) {
            let sid = ws.session_id;
            self.session_to_window.remove(&sid);
            self.session_mgr.remove(sid);
            log::info!("Closed window {window_id:?}, session {}", sid.0);
        }
    }

    /// PTY 出力があったときに Terminal の Grid から GPU バッファを更新する。
    fn redraw_session(&mut self, session_id: SessionId) {
        let Some(&window_id) = self.session_to_window.get(&session_id) else { return };
        let Some(ws) = self.windows.get_mut(&window_id) else { return };
        let Some(session) = self.session_mgr.get(session_id) else { return };

        let metrics = *self.font_ctx.metrics();
        let cell_size = [metrics.cell_width, metrics.cell_height];
        let surface_size =
            [ws.gpu.surface_config.width as f32, ws.gpu.surface_config.height as f32];
        let atlas_size_f32 = ws.atlas.size() as f32;

        let state_lock =
            session.term_state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let grid = state_lock.terminal.grid();

        let needed = grid.screen_lines() * grid.columns();
        ws.cell_pipeline.ensure_capacity(&ws.gpu.device, needed);

        ws.cell_pipeline.update_from_grid(
            &ws.gpu.queue,
            grid,
            &mut self.font_ctx,
            &mut ws.atlas,
            atlas_size_f32,
            cell_size,
            surface_size,
        );
        drop(state_lock);

        ws.atlas.upload_if_dirty(&ws.gpu.queue);
        ws.window.request_redraw();
    }

    /// ウィンドウリサイズ時に GPU・Terminal を更新する。
    fn handle_resize(&mut self, window_id: WindowId, width: u32, height: u32) {
        let Some(ws) = self.windows.get_mut(&window_id) else { return };
        ws.gpu.resize(width, height);

        let metrics = *self.font_ctx.metrics();
        let (cols, rows) =
            calc_grid_size(width as f32, height as f32, metrics.cell_width, metrics.cell_height);

        if let Some(session) = self.session_mgr.get(ws.session_id) {
            let mut state =
                session.term_state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            state.terminal.resize(rows, cols);
            drop(state);

            // PTY にもリサイズを通知して SIGWINCH を送る。
            let pty_size =
                PtySize::new(rows.try_into().unwrap_or(24), cols.try_into().unwrap_or(80));
            session.resize_pty(pty_size);
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
                // リサイズ後に再描画
                if let Some(ws) = self.windows.get(&id) {
                    let sid = ws.session_id;
                    self.redraw_session(sid);
                }
            }

            WindowEvent::ModifiersChanged(mods) => {
                self.modifiers = mods.state();
            }

            WindowEvent::KeyboardInput { event: key_event, .. } => {
                if key_event.state == ElementState::Pressed {
                    // Cmd+N (macOS) / Ctrl+Shift+N で新規ウィンドウ
                    if is_new_window_shortcut(&key_event.logical_key, self.modifiers) {
                        self.create_window(event_loop);
                        return;
                    }

                    let Some(ws) = self.windows.get(&id) else { return };
                    let Some(session) = self.session_mgr.get(ws.session_id) else { return };

                    let mode = session
                        .term_state
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .terminal
                        .mode();

                    if let Some(bytes) = key_to_bytes(&key_event.logical_key, self.modifiers, mode)
                    {
                        if let Err(e) = session.pty_io.write_tx.try_send(bytes) {
                            log::warn!("PTY write channel full or closed: {e}");
                        }
                    }
                }
            }

            WindowEvent::RedrawRequested => {
                let Some(ws) = self.windows.get(&id) else { return };
                match ws.gpu.render_frame(Some(&ws.cell_pipeline)) {
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
                    self.close_window(window_id);
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

    log::info!("SDIT starting");
    let event_loop = EventLoop::<SditEvent>::with_user_event().build().unwrap();
    let proxy = event_loop.create_proxy();
    let mut app = SditApp::new(proxy, smoke_test);
    event_loop.run_app(&mut app).unwrap();
}
