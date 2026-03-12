use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use winit::keyboard::ModifiersState;
use winit::window::{Window, WindowId};

use sdit_core::pty::{Pty, PtyConfig, PtySize};
use sdit_core::render::atlas::Atlas;
use sdit_core::render::font::FontContext;
use sdit_core::render::pipeline::{CellPipeline, GpuContext};
use sdit_core::selection::Selection;
use sdit_core::session::{
    Session, SessionId, SessionManager, SidebarState, SpawnParams, TerminalState,
};

use crate::window::{spawn_pty_reader, spawn_pty_writer};

// ---------------------------------------------------------------------------
// IME プリエディット状態
// ---------------------------------------------------------------------------

/// IME プリエディット（変換中テキスト）の状態。
#[derive(Debug, Clone)]
pub(crate) struct PreeditState {
    /// 変換中のテキスト。
    pub(crate) text: String,
    /// カーソル位置（バイトオフセットの範囲）。将来のカーソル描画に使用する。
    #[allow(dead_code)]
    pub(crate) cursor_offset: Option<(usize, usize)>,
}

// ---------------------------------------------------------------------------
// カスタムイベント型
// ---------------------------------------------------------------------------

/// winit ユーザーイベント。
#[derive(Debug)]
pub(crate) enum SditEvent {
    /// PTY から新しいデータが来た → 対象セッションのウィンドウを再描画。
    PtyOutput(SessionId),
    /// 子プロセスが終了した → 対応ウィンドウを閉じる。
    ChildExit(SessionId, i32),
    /// OSC 52 クリップボード書き込み要求。
    ClipboardWrite(String),
}

// ---------------------------------------------------------------------------
// WindowState — ウィンドウ1枚分の状態
// ---------------------------------------------------------------------------

/// ウィンドウ1枚が保持する描画コンテキストとセッション参照。
pub(crate) struct WindowState {
    pub(crate) window: Arc<Window>,
    pub(crate) gpu: GpuContext<'static>,
    pub(crate) cell_pipeline: CellPipeline,
    /// サイドバー描画用パイプライン（表示中のみ使用）。
    pub(crate) sidebar_pipeline: CellPipeline,
    pub(crate) atlas: Atlas,
    /// このウィンドウに属するセッション群（タブ順序）。
    pub(crate) sessions: Vec<SessionId>,
    /// アクティブセッションのインデックス（`sessions` 内）。
    pub(crate) active_index: usize,
    /// サイドバー状態。
    pub(crate) sidebar: SidebarState,
}

impl WindowState {
    /// アクティブセッションの `SessionId` を返す。
    ///
    /// # Panics
    ///
    /// `sessions` が空、または `active_index` が範囲外の場合にパニックする。
    /// 設計上 `sessions` は常に1つ以上のエントリを持つ不変条件が保証されている。
    pub(crate) fn active_session_id(&self) -> SessionId {
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

#[allow(clippy::struct_excessive_bools)]
pub(crate) struct SditApp {
    /// `WindowId` → ウィンドウ状態のマッピング。
    pub(crate) windows: HashMap<WindowId, WindowState>,
    /// `SessionId` → `WindowId` の逆引き（`PtyOutput` から正しいウィンドウを特定）。
    pub(crate) session_to_window: HashMap<SessionId, WindowId>,
    /// セッションマネージャ（全セッションを管理）。
    pub(crate) session_mgr: SessionManager,
    /// フォントコンテキスト（全ウィンドウで共有）。
    pub(crate) font_ctx: FontContext,
    /// 解決済みカラーテーブル。
    pub(crate) colors: sdit_core::config::color::ResolvedColors,
    /// winit modifier キーの状態。
    pub(crate) modifiers: ModifiersState,
    /// winit イベントループへのプロキシ。
    pub(crate) event_proxy: winit::event_loop::EventLoopProxy<SditEvent>,
    /// `SDIT_SMOKE_TEST=1` のとき true。1フレーム描画後に `event_loop.exit()` を呼ぶ。
    pub(crate) smoke_test: bool,
    /// 初回 resumed で最初のウィンドウを作成済みか。
    pub(crate) initialized: bool,
    /// マウスカーソルの現在位置（物理ピクセル）。
    pub(crate) cursor_position: Option<(f64, f64)>,
    /// サイドバー内ドラッグの開始行インデックス。
    pub(crate) drag_source_row: Option<usize>,
    /// テキスト選択の現在の選択範囲。
    pub(crate) selection: Option<Selection>,
    /// ターミナル領域でマウスドラッグ中かどうか。
    pub(crate) is_selecting: bool,
    /// 最後のクリック時刻（ダブル/トリプルクリック判定用）。
    pub(crate) last_click_time: Option<std::time::Instant>,
    /// 最後のクリック位置（グリッド座標）。
    pub(crate) last_click_pos: Option<(usize, usize)>,
    /// 連続クリック回数（シングル=1、ダブル=2、トリプル=3）。
    pub(crate) click_count: u8,
    /// クリップボード操作コンテキスト。
    pub(crate) clipboard: Option<arboard::Clipboard>,
    /// カーソル点滅状態（true = 表示中）。
    pub(crate) cursor_blink_visible: bool,
    /// 最後にカーソル点滅状態を切り替えた時刻。
    pub(crate) cursor_blink_last_toggle: std::time::Instant,
    /// IME プリエディット状態。
    pub(crate) preedit: Option<PreeditState>,
}

impl SditApp {
    pub(crate) fn new(
        event_proxy: winit::event_loop::EventLoopProxy<SditEvent>,
        smoke_test: bool,
        config: &sdit_core::config::Config,
    ) -> Self {
        Self {
            windows: HashMap::new(),
            session_to_window: HashMap::new(),
            session_mgr: SessionManager::new(),
            font_ctx: FontContext::from_config(&config.font),
            colors: sdit_core::config::color::ResolvedColors::from_theme(&config.colors.theme),
            modifiers: ModifiersState::empty(),
            event_proxy,
            smoke_test,
            initialized: false,
            cursor_position: None,
            drag_source_row: None,
            selection: None,
            is_selecting: false,
            last_click_time: None,
            last_click_pos: None,
            click_count: 0,
            clipboard: arboard::Clipboard::new()
                .map_err(|e| log::warn!("Clipboard init failed: {e}"))
                .ok(),
            cursor_blink_visible: true,
            cursor_blink_last_toggle: std::time::Instant::now(),
            preedit: None,
        }
    }

    /// 新しいセッションを生成して `SessionManager` に登録する。
    ///
    /// GPU パイプラインの初期化は行わない（呼び出し側で描画を更新する）。
    pub(crate) fn spawn_session(&mut self, rows: usize, cols: usize) -> Option<SessionId> {
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

                        let write_tx_for_reader = pty_write_tx.clone();
                        let reader = spawn_pty_reader(
                            pty,
                            term_state,
                            reader_proxy,
                            sid,
                            child_exited,
                            write_tx_for_reader,
                        );
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
    pub(crate) fn cascade_position(&self) -> Option<winit::dpi::PhysicalPosition<i32>> {
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
}
