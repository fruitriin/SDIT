use std::io::Read;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use sdit_core::grid::Scroll;
use sdit_core::pty::Pty;
use sdit_core::render::atlas::Atlas;
use sdit_core::render::font::FontContext;
use sdit_core::render::pipeline::CellVertex;
use sdit_core::session::{SessionId, SidebarState, TerminalState};

use crate::app::SditEvent;

// ---------------------------------------------------------------------------
// PTY リーダースレッド
// ---------------------------------------------------------------------------

pub(crate) fn spawn_pty_reader(
    mut pty: Pty,
    term_state: Arc<Mutex<TerminalState>>,
    event_proxy: winit::event_loop::EventLoopProxy<SditEvent>,
    session_id: SessionId,
    child_exited: Arc<std::sync::atomic::AtomicBool>,
    write_tx: mpsc::SyncSender<Vec<u8>>,
) -> JoinHandle<()> {
    std::thread::Builder::new()
        .name(format!("pty-reader-{}", session_id.0))
        .spawn(move || {
            let mut buf = [0u8; 8192];
            // BEL レート制限: 最後に BEL イベントを送出した時刻を記録する
            let mut last_bell_time: Option<std::time::Instant> = None;
            loop {
                match pty.read(&mut buf) {
                    Ok(0) => {
                        child_exited.store(true, std::sync::atomic::Ordering::Release);
                        let _ = event_proxy.send_event(SditEvent::ChildExit(session_id, 0));
                        break;
                    }
                    Ok(n) => {
                        // Mutex ロック内で処理し、PTY への応答バイト列はロック外で送信する。
                        // write_tx.send() はチャンネルが満杯のときブロッキングするため、
                        // Mutex を保持したままだとメインスレッドが同 Mutex を取得できず停滞する。
                        let pending_write = {
                            let mut state = term_state
                                .lock()
                                .unwrap_or_else(std::sync::PoisonError::into_inner);
                            let TerminalState { processor, terminal } = &mut *state;
                            processor.advance(terminal, &buf[..n]);
                            // Terminal からの応答（DA/DSR/CPR等）を回収。送信はロック外で行う。
                            let pending = terminal.drain_pending_writes();
                            // BEL 処理（100ms レート制限: BEL ボムによるイベントキュー枯渇を防ぐ）
                            if terminal.take_bell() {
                                let now = std::time::Instant::now();
                                let should_send = last_bell_time
                                    .is_none_or(|t| now.duration_since(t).as_millis() >= 100);
                                if should_send {
                                    log::info!("BEL received (session {})", session_id.0);
                                    let _ = event_proxy.send_event(SditEvent::BellRing(session_id));
                                    last_bell_time = Some(now);
                                }
                            }
                            // OSC 52 クリップボード書き込み処理
                            if let Some(text) = terminal.take_clipboard_write() {
                                let _ = event_proxy.send_event(SditEvent::ClipboardWrite(text));
                            }
                            // OSC 9/99 デスクトップ通知処理
                            if let Some((title, body)) = terminal.take_notification() {
                                let _ = event_proxy
                                    .send_event(SditEvent::DesktopNotification { title, body });
                            }
                            // OSC 7 CWD 変更処理
                            if let Some(cwd) = terminal.take_cwd() {
                                let _ = event_proxy
                                    .send_event(SditEvent::CwdChanged { session_id, cwd });
                            }
                            // OSC 133 コマンド終了通知処理
                            if let Some((elapsed_secs, exit_code)) =
                                terminal.take_command_finished()
                            {
                                let _ = event_proxy.send_event(SditEvent::CommandFinished {
                                    session_id,
                                    elapsed_secs,
                                    exit_code,
                                });
                            }
                            // 新しい出力があったら display_offset を 0 にリセット（ライブビュー追従）
                            if terminal.grid().display_offset() > 0 {
                                terminal.grid_mut().scroll_display(Scroll::Bottom);
                            }
                            pending
                        };
                        // Mutex ロック解放後に PTY へ応答を書き戻す（デッドロック回避）
                        if let Some(response) = pending_write {
                            let _ = write_tx.send(response);
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

pub(crate) fn spawn_pty_writer(
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
// ユーティリティ
// ---------------------------------------------------------------------------

pub(crate) fn calc_grid_size(
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
/// `session_names` にカスタム名（`Some(name)`）がある場合はそちらを優先表示する。
/// `renaming_row` が `Some((row, text))` の場合、その行をリネームモードとして `text` を表示する。
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_sidebar_cells(
    sessions: &[SessionId],
    session_names: &[Option<String>],
    renaming_row: Option<(usize, &str)>,
    active_index: usize,
    sidebar: &SidebarState,
    metrics: &sdit_core::render::font::CellMetrics,
    surface_size: [f32; 2],
    font_ctx: &mut FontContext,
    atlas: &mut Atlas,
    colors: &sdit_core::config::color::ResolvedColors,
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

        // セッション名を生成（例: "> My Session" or "  Session 1"）
        let label = if is_session_row {
            let prefix = if is_active { "> " } else { "  " };
            // リネームモード中の行はテキスト入力を表示する
            if let Some((renaming_r, text)) = renaming_row {
                if row == renaming_r {
                    format!("{prefix}{text}_")
                } else {
                    let name =
                        session_names.get(row).and_then(|n| n.as_deref()).unwrap_or_default();
                    if name.is_empty() {
                        format!("{prefix}Session {}", sessions[row].0)
                    } else {
                        format!("{prefix}{name}")
                    }
                }
            } else {
                let name = session_names.get(row).and_then(|n| n.as_deref()).unwrap_or_default();
                if name.is_empty() {
                    format!("{prefix}Session {}", sessions[row].0)
                } else {
                    format!("{prefix}{name}")
                }
            }
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
                is_color_glyph: 0.0,
            });
        }
    }

    cells
}
