use std::io::Read;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

use sdit_core::grid::Dimensions;
use sdit_core::pty::{Pty, PtyConfig, PtySize};
use sdit_core::terminal::{Processor, TermMode, Terminal};
use sdit_render::atlas::Atlas;
use sdit_render::font::FontContext;
use sdit_render::pipeline::{CellPipeline, GpuContext};

// ---------------------------------------------------------------------------
// カスタムイベント型
// ---------------------------------------------------------------------------

/// winit ユーザーイベント。
#[derive(Debug)]
pub enum SditEvent {
    /// PTY から新しいデータが来た → 再描画。
    PtyOutput,
    /// 子プロセスが終了した → アプリ終了。
    ChildExit(i32),
}

// ---------------------------------------------------------------------------
// Terminal 共有状態
// ---------------------------------------------------------------------------

/// Terminal と Processor を一緒に保護する構造体。
struct TerminalState {
    terminal: Terminal,
    processor: Processor,
}

// ---------------------------------------------------------------------------
// SditApp
// ---------------------------------------------------------------------------

struct SditApp {
    window: Option<Arc<Window>>,
    gpu: Option<GpuContext<'static>>,
    cell_pipeline: Option<CellPipeline>,
    atlas: Option<Atlas>,
    font_ctx: Option<FontContext>,
    /// Terminal + Processor（PTY リーダースレッドと共有）。
    term_state: Option<Arc<Mutex<TerminalState>>>,
    /// PTY への書き込みチャネル。
    pty_write_tx: Option<mpsc::SyncSender<Vec<u8>>>,
    /// PTY リーダースレッドのハンドル（Drop 時に join しない — フィールドに保持するだけ）。
    #[allow(clippy::used_underscore_binding)]
    _pty_reader_thread: Option<JoinHandle<()>>,
    /// PTY ライタースレッドのハンドル。
    #[allow(clippy::used_underscore_binding)]
    _pty_writer_thread: Option<JoinHandle<()>>,
    /// winit modifier キーの状態。
    modifiers: winit::keyboard::ModifiersState,
    /// winit イベントループへのプロキシ（PTY リーダースレッドに渡す）。
    event_proxy: winit::event_loop::EventLoopProxy<SditEvent>,
    /// `SDIT_SMOKE_TEST=1` のとき true。1フレーム描画後に `event_loop.exit()` を呼ぶ。
    smoke_test: bool,
}

impl SditApp {
    fn new(event_proxy: winit::event_loop::EventLoopProxy<SditEvent>, smoke_test: bool) -> Self {
        Self {
            window: None,
            gpu: None,
            cell_pipeline: None,
            atlas: None,
            font_ctx: None,
            term_state: None,
            pty_write_tx: None,
            _pty_reader_thread: None,
            _pty_writer_thread: None,
            modifiers: winit::keyboard::ModifiersState::empty(),
            event_proxy,
            smoke_test,
        }
    }

    /// GPU・フォント・アトラス・パイプラインを初期化し、PTY を起動する。
    fn init(&mut self) {
        let event_proxy = self.event_proxy.clone();
        let Some(gpu) = &self.gpu else { return };

        // ----- フォントコンテキスト -----
        let mut font_ctx = FontContext::new(14.0, 1.2);
        let metrics = *font_ctx.metrics();

        // ----- Grid サイズをウィンドウとフォントメトリクスから計算 -----
        let (cols, rows) = calc_grid_size(
            gpu.surface_config.width as f32,
            gpu.surface_config.height as f32,
            metrics.cell_width,
            metrics.cell_height,
        );

        // ----- Terminal + Processor -----
        let terminal = Terminal::new(rows, cols, 10_000);
        let processor = Processor::new();
        let term_state = Arc::new(Mutex::new(TerminalState { terminal, processor }));

        // ----- PTY 起動 -----
        let pty_size = PtySize::new(rows.try_into().unwrap_or(24), cols.try_into().unwrap_or(80));
        let mut pty_config = PtyConfig::default();
        pty_config.env.insert("TERM".to_owned(), "xterm-256color".to_owned());

        let pty = match Pty::spawn(&pty_config, pty_size) {
            Ok(p) => p,
            Err(e) => {
                log::error!("PTY spawn failed: {e}");
                return;
            }
        };

        // ----- PTY の writer fd をクローンして read/write スレッドを分離 -----
        let pty_writer = match pty.try_clone_writer() {
            Ok(w) => w,
            Err(e) => {
                log::error!("PTY writer clone failed: {e}");
                return;
            }
        };
        let (pty_write_tx, pty_write_rx) = mpsc::sync_channel::<Vec<u8>>(64);
        let writer_event_proxy = event_proxy.clone();
        let pty_reader_thread = spawn_pty_reader(pty, Arc::clone(&term_state), event_proxy);
        let pty_writer_thread = spawn_pty_writer(pty_writer, pty_write_rx, writer_event_proxy);

        // ----- テクスチャアトラス -----
        let mut atlas = Atlas::new(&gpu.device, 512);

        // ----- CellPipeline -----
        let cell_size = [metrics.cell_width, metrics.cell_height];
        let surface_size = [gpu.surface_config.width as f32, gpu.surface_config.height as f32];
        let atlas_size_f32 = atlas.size() as f32;

        let state_lock = term_state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let grid = state_lock.terminal.grid();
        let mut cell_pipeline =
            CellPipeline::new(&gpu.device, gpu.surface_config.format, &atlas, rows * cols);
        cell_pipeline.update_from_grid(
            &gpu.queue,
            grid,
            &mut font_ctx,
            &mut atlas,
            atlas_size_f32,
            cell_size,
            surface_size,
        );
        drop(state_lock);

        atlas.upload_if_dirty(&gpu.queue);

        // ----- 状態を保存 -----
        self.font_ctx = Some(font_ctx);
        self.atlas = Some(atlas);
        self.cell_pipeline = Some(cell_pipeline);
        self.term_state = Some(term_state);
        self.pty_write_tx = Some(pty_write_tx);
        self._pty_reader_thread = Some(pty_reader_thread);
        self._pty_writer_thread = Some(pty_writer_thread);
    }

    /// PTY 出力があったときに Terminal の Grid から GPU バッファを更新する。
    fn redraw_from_terminal(&mut self) {
        let (Some(gpu), Some(term_state), Some(font_ctx), Some(atlas), Some(cell_pipeline)) = (
            &self.gpu,
            &self.term_state,
            &mut self.font_ctx,
            &mut self.atlas,
            &mut self.cell_pipeline,
        ) else {
            return;
        };

        let metrics = *font_ctx.metrics();
        let cell_size = [metrics.cell_width, metrics.cell_height];
        let surface_size = [gpu.surface_config.width as f32, gpu.surface_config.height as f32];
        let atlas_size_f32 = atlas.size() as f32;

        let state_lock = term_state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let grid = state_lock.terminal.grid();

        // Grid サイズが変わっていれば vertex_buffer を拡張する。
        let needed = grid.screen_lines() * grid.columns();
        cell_pipeline.ensure_capacity(&gpu.device, needed);

        cell_pipeline.update_from_grid(
            &gpu.queue,
            grid,
            font_ctx,
            atlas,
            atlas_size_f32,
            cell_size,
            surface_size,
        );
        drop(state_lock);
    }

    /// ウィンドウリサイズ時に PTY・Terminal・GPU を更新する。
    fn handle_resize(&mut self, width: u32, height: u32) {
        // GPU サーフェスをリサイズ。
        if let Some(gpu) = &mut self.gpu {
            gpu.resize(width, height);
        }

        let (Some(font_ctx), Some(term_state), Some(pty_write_tx)) =
            (&self.font_ctx, &self.term_state, &self.pty_write_tx)
        else {
            return;
        };

        let metrics = *font_ctx.metrics();
        let (cols, rows) =
            calc_grid_size(width as f32, height as f32, metrics.cell_width, metrics.cell_height);

        // Terminal をリサイズ。
        {
            let mut state = term_state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            state.terminal.resize(rows, cols);
        }

        // PTY に SIGWINCH を送る（サイズ変更を書き込みチャネル経由で通知する代わりに
        // PtySize をメインスレッドから直接送れないため、特殊なエスケープシーケンスを
        // 使わず、リサイズパケットを送信する）。
        // Note: pty_write_tx は `Vec<u8>` のチャネルなので PTY リサイズは別途行う。
        // ここでは PTY の resize を行うため、pty_write_tx に特殊なメッセージを
        // 送るのではなく、Pty::resize を呼べる仕組みが必要。
        // 現設計では PTY はリーダースレッドが所有しているので、
        // リサイズ要求をチャネルで送るようにする。
        // → 簡易実装として、リサイズは winit スレッドからは行わず
        //   PtyOutput イベントで次の描画時に自動的に追従する。
        // TODO: より正確には Pty::resize をリーダースレッドが受け取れるよう拡張する。
        let _ = pty_write_tx; // 現時点では直接リサイズ通知は不要
    }
}

// ---------------------------------------------------------------------------
// ApplicationHandler
// ---------------------------------------------------------------------------

impl ApplicationHandler<SditEvent> for SditApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attrs = Window::default_attributes()
            .with_title("SDIT")
            .with_inner_size(winit::dpi::LogicalSize::new(800.0_f64, 600.0_f64));

        let window = Arc::new(event_loop.create_window(attrs).unwrap());
        let gpu = GpuContext::new(&window).unwrap();

        self.window = Some(window);
        self.gpu = Some(gpu);

        self.init();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Resized(size) => {
                self.handle_resize(size.width, size.height);
                self.redraw_from_terminal();
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }

            WindowEvent::ModifiersChanged(mods) => {
                self.modifiers = mods.state();
            }

            WindowEvent::KeyboardInput { event: key_event, .. } => {
                if key_event.state == ElementState::Pressed {
                    // APP_CURSOR モードを取得する。
                    let mode = self
                        .term_state
                        .as_ref()
                        .map(|ts| {
                            ts.lock()
                                .unwrap_or_else(std::sync::PoisonError::into_inner)
                                .terminal
                                .mode()
                        })
                        .unwrap_or_default();
                    if let Some(bytes) = key_to_bytes(&key_event.logical_key, self.modifiers, mode)
                    {
                        if let Some(tx) = &self.pty_write_tx {
                            if let Err(e) = tx.try_send(bytes) {
                                log::warn!("PTY write channel full or closed: {e}");
                            }
                        }
                    }
                }
            }

            WindowEvent::RedrawRequested => {
                if let Some(gpu) = &self.gpu {
                    match gpu.render_frame(self.cell_pipeline.as_ref()) {
                        Ok(()) => {}
                        Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                            if let Some(gpu) = &mut self.gpu {
                                let (w, h) = (gpu.surface_config.width, gpu.surface_config.height);
                                gpu.resize(w, h);
                            }
                        }
                        Err(e) => log::error!("Render error: {e}"),
                    }
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
            SditEvent::PtyOutput => {
                self.redraw_from_terminal();
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            SditEvent::ChildExit(code) => {
                log::info!("Child process exited with code {code}");
                event_loop.exit();
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // PtyOutput イベント駆動で描画するため、ここでは何もしない。
    }
}

// ---------------------------------------------------------------------------
// PTY リーダースレッド
// ---------------------------------------------------------------------------

/// PTY からの読み取り専用スレッド。
///
/// PTY 出力を Terminal に流し、winit イベントループに再描画を通知する。
/// 書き込みは別スレッド (`spawn_pty_writer`) で行うため、
/// read のブロッキングが write をブロックしない。
fn spawn_pty_reader(
    mut pty: Pty,
    term_state: Arc<Mutex<TerminalState>>,
    event_proxy: winit::event_loop::EventLoopProxy<SditEvent>,
) -> JoinHandle<()> {
    std::thread::Builder::new()
        .name("pty-reader".into())
        .spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match pty.read(&mut buf) {
                    Ok(0) => {
                        let _ = event_proxy.send_event(SditEvent::ChildExit(0));
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
                        let _ = event_proxy.send_event(SditEvent::PtyOutput);
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(1));
                    }
                    Err(e) if e.raw_os_error() == Some(5) => {
                        // EIO: PTY が閉じられた（子プロセス終了）。
                        let _ = event_proxy.send_event(SditEvent::ChildExit(0));
                        break;
                    }
                    Err(e) => {
                        log::error!("PTY read error: {e}");
                        let _ = event_proxy.send_event(SditEvent::ChildExit(1));
                        break;
                    }
                }
            }
        })
        .unwrap()
}

/// PTY への書き込み専用スレッド。
///
/// GUI スレッドからのキー入力データをチャネル経由で受け取り、
/// クローンした PTY master fd に書き込む。
/// チャネルが閉じられたら（GUI 終了時）スレッドを終了する。
fn spawn_pty_writer(
    mut writer: std::fs::File,
    pty_write_rx: mpsc::Receiver<Vec<u8>>,
    event_proxy: winit::event_loop::EventLoopProxy<SditEvent>,
) -> JoinHandle<()> {
    std::thread::Builder::new()
        .name("pty-writer".into())
        .spawn(move || {
            // recv() はチャネルが閉じられるまでブロッキングで待機する。
            // GUI スレッドが pty_write_tx を drop すると RecvError で終了する。
            while let Ok(data) = pty_write_rx.recv() {
                if let Err(e) = std::io::Write::write_all(&mut writer, &data) {
                    log::error!("PTY write error: {e}");
                    let _ = event_proxy.send_event(SditEvent::ChildExit(1));
                    break;
                }
            }
        })
        .unwrap()
}

// ---------------------------------------------------------------------------
// キー入力 → PTY バイト列変換
// ---------------------------------------------------------------------------

/// winit `Key` → PTY に送るバイト列。
///
/// Arrow キーは `mode` の `APP_CURSOR` フラグに応じてシーケンスを切り替える。
/// 変換できない場合は `None`。
fn key_to_bytes(
    key: &Key,
    modifiers: winit::keyboard::ModifiersState,
    mode: TermMode,
) -> Option<Vec<u8>> {
    match key {
        Key::Character(s) => {
            let bytes = s.as_bytes();
            if bytes.is_empty() {
                return None;
            }
            // Ctrl+a-z: 0x01-0x1a
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

/// ウィンドウサイズとセルサイズからグリッドの (cols, rows) を計算する。
///
/// 最低 1×1 を保証する。
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

/// Grid の全行を走査して `needle` を含む行があれば `true` を返す。
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

/// `--headless` モード: PTY を spawn して出力を Terminal に流し、
/// `SDIT_HEADLESS_OK` が Grid に現れたら exit(0)、タイムアウトで exit(1)。
fn run_headless() -> ! {
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
                // EOF: 子プロセス終了後も Grid を最終確認。
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
                // EIO: PTY が閉じられた（子プロセス終了）。Grid を最終確認。
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

    // --headless: winit/wgpu を使わず PTY → Terminal の動作確認のみ行う。
    if std::env::args().any(|a| a == "--headless") {
        log::info!("SDIT starting in headless mode");
        run_headless();
    }

    // SDIT_SMOKE_TEST=1: 1フレーム描画後に正常終了する（debug ビルドのみ有効）。
    let smoke_test =
        cfg!(debug_assertions) && std::env::var("SDIT_SMOKE_TEST").as_deref() == Ok("1");

    log::info!("SDIT starting");
    let event_loop = EventLoop::<SditEvent>::with_user_event().build().unwrap();
    let proxy = event_loop.create_proxy();
    let mut app = SditApp::new(proxy, smoke_test);
    event_loop.run_app(&mut app).unwrap();
}
