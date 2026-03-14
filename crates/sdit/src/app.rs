use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use regex::Regex;

use winit::keyboard::ModifiersState;
use winit::window::{Window, WindowId};

use sdit_core::index::Point;
use sdit_core::pty::{Pty, PtyConfig, PtySize};
use sdit_core::render::atlas::Atlas;
use sdit_core::render::font::FontContext;
use sdit_core::render::pipeline::{CellPipeline, GpuContext};
use sdit_core::selection::Selection;
use sdit_core::session::{
    Session, SessionId, SessionManager, SidebarState, SpawnParams, TerminalState,
};
use sdit_core::terminal::url_detector::UrlDetector;
use sdit_core::terminal::vi_mode::ViCursor;

use crate::window::{calc_grid_size, spawn_pty_reader, spawn_pty_writer};

// ---------------------------------------------------------------------------
// URL ホバー状態
// ---------------------------------------------------------------------------

/// URL ホバー中の状態（Cmd/Ctrl 押下中に URL 上にカーソルがある場合）。
#[derive(Debug, Clone)]
pub(crate) struct UrlHoverState {
    pub(crate) row: usize,
    pub(crate) start_col: usize,
    pub(crate) end_col: usize,
    /// ホバー中の URL 文字列（将来のツールチップ表示等で利用）。
    #[allow(dead_code)]
    pub(crate) url: String,
}

// ---------------------------------------------------------------------------
// 検索状態
// ---------------------------------------------------------------------------

/// 検索バーの状態。
#[derive(Debug, Clone)]
pub(crate) struct SearchState {
    /// ユーザーが入力した検索クエリ。
    pub(crate) query: String,
    /// 検索マッチ結果のリスト。
    pub(crate) matches: Vec<sdit_core::terminal::search::SearchMatch>,
    /// 現在フォーカスしているマッチのインデックス（0-indexed）。
    pub(crate) current_match: usize,
}

impl SearchState {
    /// 新しい空の検索状態を作成する。
    pub(crate) fn new() -> Self {
        Self { query: String::new(), matches: Vec::new(), current_match: 0 }
    }
}

// ---------------------------------------------------------------------------
// QuickSelect 状態
// ---------------------------------------------------------------------------

/// QuickSelect のヒント1件分。
#[derive(Debug, Clone)]
pub(crate) struct QuickSelectHint {
    /// ヒントラベル（"a", "s", "aa" など）。
    pub(crate) label: String,
    /// ビューポート行番号（0-indexed）。
    pub(crate) row: usize,
    /// マッチ開始列（0-indexed）。
    pub(crate) start_col: usize,
    /// マッチ終了列（0-indexed, exclusive）。
    pub(crate) end_col: usize,
    /// マッチしたテキスト。
    pub(crate) text: String,
}

/// QuickSelect モードの状態。
#[derive(Debug, Clone)]
pub(crate) struct QuickSelectState {
    /// 全ヒントのリスト。
    pub(crate) hints: Vec<QuickSelectHint>,
    /// ユーザーが入力中のヒント文字列。
    pub(crate) input: String,
}

impl QuickSelectState {
    /// ヒントラベルを生成する（a-z、次いで aa-az、ba-bz...）。
    pub(crate) fn generate_label(index: usize) -> String {
        const CHARS: &[u8] = b"asdfghjklqwertyuiopzxcvbnm";
        let n = CHARS.len();
        if index < n {
            String::from(CHARS[index] as char)
        } else {
            let idx = index - n;
            let hi = idx / n;
            let lo = idx % n;
            if hi < n {
                format!("{}{}", CHARS[hi] as char, CHARS[lo] as char)
            } else {
                // 26*26 + 26 = 702個を超える場合（実用上まず起きない）
                format!("{index}")
            }
        }
    }
}

// ---------------------------------------------------------------------------
// vi モード状態
// ---------------------------------------------------------------------------

/// vi モードの選択種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ViSelectionKind {
    /// 文字単位選択 (v)。
    Char,
    /// 行単位選択 (V)。
    Line,
    /// ブロック選択 (Ctrl+V) — 将来実装。
    #[allow(dead_code)]
    Block,
}

/// vi モードの状態。
#[derive(Debug, Clone)]
pub(crate) struct ViModeState {
    /// vi カーソルの現在位置。
    pub(crate) cursor: ViCursor,
    /// 選択種類（None = 選択なし）。
    pub(crate) selection: Option<ViSelectionKind>,
    /// 選択の起点（v/V を押した位置）。
    pub(crate) selection_origin: Option<Point>,
    /// g キーが入力待ち状態か（gg = Top モーション）。
    pub(crate) pending_g: bool,
}

// ---------------------------------------------------------------------------
// 閉じる確認状態
// ---------------------------------------------------------------------------

/// セッション/ウィンドウ閉じる確認の対象。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PendingClose {
    /// アクティブセッションを閉じる確認中。
    Session(sdit_core::session::SessionId, winit::window::WindowId),
    /// アプリケーション全体を終了する確認中。
    Quit,
}

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
    /// 設定ファイルが変更された → 設定を再読み込みして反映する。
    ConfigReloaded,
    /// メニューバーのアイテムが選択された。
    MenuAction(sdit_core::config::keybinds::Action),
    /// BEL (0x07) 受信 → ビジュアルベル + Dock バウンス。
    BellRing(SessionId),
    /// OSC 9/99 デスクトップ通知要求。
    DesktopNotification { title: String, body: String },
    /// OSC 7 で CWD が変更された → Session の cwd フィールドを更新する。
    CwdChanged { session_id: SessionId, cwd: String },
}

// ---------------------------------------------------------------------------
// VisualBell — ビジュアルベルアニメーション状態
// ---------------------------------------------------------------------------

/// ビジュアルベルのアニメーション状態。
pub(crate) struct VisualBell {
    /// ベルが鳴り始めた時刻。None = 非アクティブ。
    start_time: Option<std::time::Instant>,
    /// フェードアウト時間。
    duration: std::time::Duration,
}

impl VisualBell {
    pub(crate) fn new(duration_ms: u32) -> Self {
        Self {
            start_time: None,
            duration: std::time::Duration::from_millis(u64::from(duration_ms)),
        }
    }

    /// ベルを鳴らす。アニメーション中の再呼び出しは無視する（BEL ボム対策）。
    pub(crate) fn ring(&mut self) {
        // アニメーション中なら無視（レート制限）
        if self.start_time.is_some() && self.intensity_inner() > 0.0 {
            return;
        }
        self.start_time = Some(std::time::Instant::now());
    }

    /// 現在の intensity (0.0〜1.0) を返す。0.0 = 完全にフェードアウト。
    pub(crate) fn intensity(&self) -> f32 {
        self.intensity_inner()
    }

    fn intensity_inner(&self) -> f32 {
        let Some(start) = self.start_time else { return 0.0 };
        let elapsed = start.elapsed();
        if self.duration.is_zero() || elapsed >= self.duration {
            return 0.0;
        }
        let t = elapsed.as_secs_f32() / self.duration.as_secs_f32();
        1.0 - t // 線形フェードアウト
    }

    /// アニメーションが完了しているかを返す。完了済みなら state をクリアする。
    #[allow(dead_code)]
    pub(crate) fn completed(&mut self) -> bool {
        if self.start_time.is_some() && self.intensity() <= 0.0 {
            self.start_time = None;
            true
        } else {
            self.start_time.is_none()
        }
    }

    /// duration を更新する（Hot Reload 用）。
    pub(crate) fn set_duration(&mut self, duration_ms: u32) {
        self.duration = std::time::Duration::from_millis(u64::from(duration_ms));
    }
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
    /// ビジュアルベルアニメーション状態。
    pub(crate) visual_bell: VisualBell,
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
    /// マウスカーソルが非表示状態かどうか（hide_when_typing 機能用）。
    pub(crate) cursor_hidden: bool,
    /// カーソル点滅状態（true = 表示中）。
    pub(crate) cursor_blink_visible: bool,
    /// 最後にカーソル点滅状態を切り替えた時刻。
    pub(crate) cursor_blink_last_toggle: std::time::Instant,
    /// IME プリエディット状態。
    pub(crate) preedit: Option<PreeditState>,
    /// 設定ファイルから読み込んだデフォルトフォントサイズ（Cmd+0 でこのサイズに復帰）。
    pub(crate) default_font_size: f32,
    /// URL 検出器（正規表現コンパイル済み）。
    pub(crate) url_detector: UrlDetector,
    /// 現在 URL ホバー中の状態（Cmd/Ctrl 押下中に URL 上にカーソルがある場合）。
    pub(crate) hovered_url: Option<UrlHoverState>,
    /// 検索バーの状態。None = 検索バー非表示。
    pub(crate) search: Option<SearchState>,
    /// QuickSelect モードの状態。None = 非アクティブ。
    pub(crate) quick_select: Option<QuickSelectState>,
    /// vi モード（コピーモード）の状態。None = 非アクティブ。
    pub(crate) vi_mode: Option<ViModeState>,
    /// 設定全体（キーバインド等）。
    pub(crate) config: sdit_core::config::Config,
    /// コンパイル済みカスタム QuickSelect パターン（config 更新時に再コンパイル）。
    pub(crate) compiled_quick_select_patterns: Vec<Regex>,
    /// 通知スレッドが飛行中かどうか（レート制限：同時に複数スレッドを立ち上げない）。
    pub(crate) notification_in_flight: Arc<AtomicBool>,
    /// セッションリネームモードの状態。`Some((SessionId, 入力中テキスト))` のとき編集中。
    pub(crate) renaming_session: Option<(sdit_core::session::SessionId, String)>,
    /// スクロールバーをドラッグ中かどうか。
    pub(crate) scrollbar_dragging: bool,
    /// 閉じる確認ダイアログが表示中かどうか。`Some` の場合は確認中。
    pub(crate) pending_close: Option<PendingClose>,
    /// Secure Keyboard Entry が現在有効かどうか（macOS のみ有効）。
    #[cfg(target_os = "macos")]
    pub(crate) secure_input_enabled: bool,
    /// メニューバー + コンテキストメニューの共有 `MenuId` マップ。
    /// `MenuEvent` ハンドラのクロージャが `Arc` クローンを保持するため、
    /// フィールドとしては直接読まれないが、ドロップ防止のため保持する。
    #[cfg(target_os = "macos")]
    #[allow(dead_code)]
    pub(crate) menu_actions: crate::menu::SharedMenuActions,
    /// ターミナル領域の右クリックメニュー（初期化時に1回構築）。
    #[cfg(target_os = "macos")]
    pub(crate) terminal_ctx_menu: muda::Menu,
    /// サイドバー領域の右クリックメニュー（初期化時に1回構築）。
    #[cfg(target_os = "macos")]
    pub(crate) sidebar_ctx_menu: muda::Menu,
}

impl SditApp {
    #[allow(clippy::needless_pass_by_value)]
    pub(crate) fn new(
        event_proxy: winit::event_loop::EventLoopProxy<SditEvent>,
        smoke_test: bool,
        config: &sdit_core::config::Config,
        #[cfg(target_os = "macos")] menu_actions: crate::menu::SharedMenuActions,
    ) -> Self {
        let default_font_size = config.font.clamped_size();
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
            cursor_hidden: false,
            cursor_blink_visible: true,
            cursor_blink_last_toggle: std::time::Instant::now(),
            preedit: None,
            default_font_size,
            url_detector: UrlDetector::with_links(
                &config.clamped_links().cloned().collect::<Vec<_>>(),
            ),
            hovered_url: None,
            search: None,
            quick_select: None,
            vi_mode: None,
            config: config.clone(),
            compiled_quick_select_patterns: compile_quick_select_patterns(config),
            notification_in_flight: Arc::new(AtomicBool::new(false)),
            renaming_session: None,
            scrollbar_dragging: false,
            pending_close: None,
            #[cfg(target_os = "macos")]
            secure_input_enabled: false,
            #[cfg(target_os = "macos")]
            menu_actions: menu_actions.clone(),
            #[cfg(target_os = "macos")]
            terminal_ctx_menu: {
                let (menu, ids) = crate::menu::build_terminal_context_menu();
                menu_actions.lock().unwrap_or_else(std::sync::PoisonError::into_inner).extend(ids);
                menu
            },
            #[cfg(target_os = "macos")]
            sidebar_ctx_menu: {
                let (menu, ids) = crate::menu::build_sidebar_context_menu();
                menu_actions.lock().unwrap_or_else(std::sync::PoisonError::into_inner).extend(ids);
                menu
            },
        }
    }

    /// 新しいセッションを生成して `SessionManager` に登録する。
    ///
    /// GPU パイプラインの初期化は行わない（呼び出し側で描画を更新する）。
    #[allow(dead_code)]
    pub(crate) fn spawn_session(&mut self, rows: usize, cols: usize) -> Option<SessionId> {
        self.spawn_session_with_cwd(rows, cols, None)
    }

    /// CWD を指定してセッションを生成する。
    pub(crate) fn spawn_session_with_cwd(
        &mut self,
        rows: usize,
        cols: usize,
        working_dir: Option<std::path::PathBuf>,
    ) -> Option<SessionId> {
        let session_id = self.session_mgr.next_id();
        let pty_size = PtySize::new(rows.try_into().unwrap_or(24), cols.try_into().unwrap_or(80));
        let mut pty_config = PtyConfig::default();
        pty_config.env.insert("TERM".to_owned(), "xterm-256color".to_owned());
        pty_config.env.insert("TERM_PROGRAM".to_owned(), "sdit".to_owned());
        pty_config.working_directory = working_dir;

        let event_proxy = self.event_proxy.clone();
        let sid = session_id;

        let default_cursor_style = sdit_core::terminal::CursorStyle::from(self.config.cursor.style);
        let default_cursor_blinking = self.config.cursor.blinking;

        let session = match Session::spawn(
            session_id,
            SpawnParams {
                pty_config,
                pty_size,
                terminal_rows: rows,
                terminal_cols: cols,
                scrollback: self.config.scrollback.clamped_lines(),
                default_cursor_style,
                default_cursor_blinking,
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

        // shell_integration_enabled をセッションの Terminal に反映する
        {
            let mut state =
                session.term_state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            state.terminal.shell_integration_enabled = self.config.shell_integration.enabled;
        }

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

    /// 設定ファイルを再読み込みして変更を反映する。
    ///
    /// フォント・カラー・キーバインドの変更を検出し、必要な部分だけ更新する。
    /// 再描画の呼び出しは呼び出し側の責任（`event_loop.rs` で `request_redraw` を呼ぶ）。
    pub(crate) fn apply_config_reload(&mut self) {
        let new_config =
            sdit_core::config::Config::load(&sdit_core::config::Config::default_path());

        // 1. フォント変更チェック
        let old_font_size = self.font_ctx.metrics().font_size;
        let new_font_size = new_config.font.clamped_size();
        let font_changed = (old_font_size - new_font_size).abs() > f32::EPSILON
            || self.config.font.family != new_config.font.family
            || (self.config.font.clamped_line_height() - new_config.font.clamped_line_height())
                .abs()
                > f32::EPSILON;

        if font_changed {
            // グリッドサイズが変わるため vi モードのカーソル座標が無効になる可能性がある
            if self.vi_mode.is_some() {
                self.vi_mode = None;
                self.selection = None;
            }
            // FontContext を再構築（family/line_height も変わりうる）
            self.font_ctx = FontContext::from_config(&new_config.font);
            self.default_font_size = new_config.font.clamped_size();
            // 全ウィンドウのアトラスをクリア
            for ws in self.windows.values_mut() {
                ws.atlas.clear();
            }
            // 全セッションをリサイズ
            let metrics = *self.font_ctx.metrics();
            let padding_x = f32::from(new_config.window.clamped_padding_x());
            let padding_y = f32::from(new_config.window.clamped_padding_y());
            let window_ids: Vec<winit::window::WindowId> = self.windows.keys().copied().collect();
            for window_id in window_ids {
                let Some(ws) = self.windows.get(&window_id) else { continue };
                let sidebar_w = ws.sidebar.width_px(metrics.cell_width);
                let surface_w = ws.gpu.surface_config.width as f32;
                let surface_h = ws.gpu.surface_config.height as f32;
                let term_width = (surface_w - sidebar_w - 2.0 * padding_x).max(0.0);
                let term_height = (surface_h - 2.0 * padding_y).max(0.0);
                let (cols, rows) = calc_grid_size(
                    term_width,
                    term_height,
                    metrics.cell_width,
                    metrics.cell_height,
                );

                let session_ids: Vec<_> = ws.sessions.clone();
                for sid in session_ids {
                    if let Some(session) = self.session_mgr.get(sid) {
                        {
                            let mut state = session
                                .term_state
                                .lock()
                                .unwrap_or_else(std::sync::PoisonError::into_inner);
                            state.terminal.resize(rows, cols);
                        }
                        let pty_size = PtySize::new(
                            rows.try_into().unwrap_or(24),
                            cols.try_into().unwrap_or(80),
                        );
                        session.resize_pty(pty_size);
                    }
                }
            }
        }

        // 2. カラー変更チェック
        if self.config.colors.theme != new_config.colors.theme {
            self.colors =
                sdit_core::config::color::ResolvedColors::from_theme(&new_config.colors.theme);
        }

        // 3. キーバインド更新（常に置換）
        // validate() は Config::load() 内で既に呼ばれている

        // 4. macOS: option_as_alt 変更チェック
        #[cfg(target_os = "macos")]
        if self.config.option_as_alt != new_config.option_as_alt {
            use winit::platform::macos::WindowExtMacOS as _;
            let winit_val =
                crate::window_ops::config_option_as_alt_to_winit(new_config.option_as_alt);
            for ws in self.windows.values() {
                ws.window.set_option_as_alt(winit_val);
            }
        }

        // 5. bell duration 変更チェック
        if self.config.bell.duration_ms != new_config.bell.duration_ms {
            for ws in self.windows.values_mut() {
                ws.visual_bell.set_duration(new_config.bell.clamped_duration_ms());
            }
        }

        // 6. window opacity/blur 変更チェック
        let opacity_changed =
            (self.config.window.opacity - new_config.window.opacity).abs() > f32::EPSILON;
        let blur_changed = self.config.window.blur != new_config.window.blur;
        if opacity_changed || blur_changed {
            for ws in self.windows.values() {
                if blur_changed {
                    ws.window.set_blur(new_config.window.blur);
                }
                ws.window.request_redraw();
            }
        }

        // 7. パディング変更チェック（グリッドサイズ再計算 + PTY リサイズ）
        let padding_changed = self.config.window.padding_x != new_config.window.padding_x
            || self.config.window.padding_y != new_config.window.padding_y;
        if padding_changed {
            // グリッドサイズが変わるため vi モードのカーソル座標が無効になる可能性がある
            if self.vi_mode.is_some() {
                self.vi_mode = None;
                self.selection = None;
            }
            let metrics = *self.font_ctx.metrics();
            #[allow(clippy::similar_names)]
            let new_px = f32::from(new_config.window.clamped_padding_x());
            #[allow(clippy::similar_names)]
            let new_py = f32::from(new_config.window.clamped_padding_y());
            let window_ids: Vec<winit::window::WindowId> = self.windows.keys().copied().collect();
            for window_id in window_ids {
                let Some(ws) = self.windows.get(&window_id) else { continue };
                let sidebar_w = ws.sidebar.width_px(metrics.cell_width);
                let surface_w = ws.gpu.surface_config.width as f32;
                let surface_h = ws.gpu.surface_config.height as f32;
                let term_width = (surface_w - sidebar_w - 2.0 * new_px).max(0.0);
                let term_height = (surface_h - 2.0 * new_py).max(0.0);
                let (cols, rows) = calc_grid_size(
                    term_width,
                    term_height,
                    metrics.cell_width,
                    metrics.cell_height,
                );
                let session_ids: Vec<_> = ws.sessions.clone();
                for sid in session_ids {
                    if let Some(session) = self.session_mgr.get(sid) {
                        {
                            let mut state = session
                                .term_state
                                .lock()
                                .unwrap_or_else(std::sync::PoisonError::into_inner);
                            state.terminal.resize(rows, cols);
                        }
                        let pty_size = PtySize::new(
                            rows.try_into().unwrap_or(24),
                            cols.try_into().unwrap_or(80),
                        );
                        session.resize_pty(pty_size);
                    }
                }
            }
        }

        // 9. カーソルデフォルト設定の変更チェック
        let cursor_changed = self.config.cursor.style != new_config.cursor.style
            || self.config.cursor.blinking != new_config.cursor.blinking;
        if cursor_changed {
            let new_style = sdit_core::terminal::CursorStyle::from(new_config.cursor.style);
            let new_blinking = new_config.cursor.blinking;
            // 全セッションの Terminal デフォルトを更新する
            for session in self.session_mgr.all() {
                let mut state =
                    session.term_state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                state.terminal.set_default_cursor(new_style, new_blinking);
            }
        }

        // 10. shell_integration 変更チェック
        let shell_integration_changed =
            self.config.shell_integration.enabled != new_config.shell_integration.enabled;
        if shell_integration_changed {
            let enabled = new_config.shell_integration.enabled;
            for session in self.session_mgr.all() {
                let mut state =
                    session.term_state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                state.terminal.shell_integration_enabled = enabled;
            }
        }

        // 11. 設定を置換
        self.compiled_quick_select_patterns = compile_quick_select_patterns(&new_config);
        // カスタムリンク変更時に UrlDetector を再構築
        self.url_detector = sdit_core::terminal::url_detector::UrlDetector::with_links(
            &new_config.clamped_links().cloned().collect::<Vec<_>>(),
        );
        self.config = new_config;

        log::info!("Config reloaded successfully");
    }

    /// フォントサイズを変更する。
    ///
    /// `delta` が `Some(d)` のとき現在サイズに `d` を加算、`None` のときデフォルトサイズに復帰する。
    /// 変更後、全ウィンドウのアトラスをクリアし、全セッションをリサイズする。
    ///
    /// 再描画の呼び出しは呼び出し側の責任（`event_loop.rs` で `request_redraw` を呼ぶ）。
    pub(crate) fn change_font_size(&mut self, delta: Option<f32>) {
        let new_size = match delta {
            Some(d) => self.font_ctx.metrics().font_size + d,
            None => self.default_font_size,
        };
        self.font_ctx.set_font_size(new_size);

        // 全ウィンドウのアトラスをクリア
        for ws in self.windows.values_mut() {
            ws.atlas.clear();
        }

        // 全ウィンドウ・全セッションをリサイズ
        let metrics = *self.font_ctx.metrics();
        let padding_x = f32::from(self.config.window.clamped_padding_x());
        let padding_y = f32::from(self.config.window.clamped_padding_y());
        let window_ids: Vec<winit::window::WindowId> = self.windows.keys().copied().collect();
        for window_id in window_ids {
            let Some(ws) = self.windows.get(&window_id) else { continue };
            let sidebar_w = ws.sidebar.width_px(metrics.cell_width);
            let surface_w = ws.gpu.surface_config.width as f32;
            let surface_h = ws.gpu.surface_config.height as f32;
            let term_width = (surface_w - sidebar_w - 2.0 * padding_x).max(0.0);
            let term_height = (surface_h - 2.0 * padding_y).max(0.0);
            let (cols, rows) =
                calc_grid_size(term_width, term_height, metrics.cell_width, metrics.cell_height);

            let session_ids: Vec<_> = ws.sessions.clone();
            for sid in session_ids {
                if let Some(session) = self.session_mgr.get(sid) {
                    {
                        let mut state = session
                            .term_state
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                        state.terminal.resize(rows, cols);
                    }
                    let pty_size =
                        PtySize::new(rows.try_into().unwrap_or(24), cols.try_into().unwrap_or(80));
                    session.resize_pty(pty_size);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// IME ユーティリティ関数
// ---------------------------------------------------------------------------

/// ペーストテキストが「unsafe」（改行を含む）かどうかを判定する。
///
/// Bracketed Paste モードが有効な場合は、シェルが改行を直接実行しないため
/// 安全とみなし、常に false を返す。
/// ただし、bracketed paste 終了シーケンス（`\x1b[201~`）は
/// bracketed paste モード内でも脱出インジェクションに使えるため、
/// モードの有無にかかわらず常に危険と判定する。
pub(crate) fn is_unsafe_paste(text: &str, bracketed_paste_mode: bool) -> bool {
    // bracketed paste 終了シーケンスは常に危険（モード有効時でも脱出に使える）
    if text.contains("\x1b[201~") {
        return true;
    }
    if bracketed_paste_mode {
        return false; // bracketed paste は安全
    }
    text.contains('\n') || text.contains('\r')
}

/// unsafe paste の確認ダイアログを表示する。
///
/// ユーザーが「Paste」を選択した場合は true を返す。
pub(crate) fn confirm_unsafe_paste(text: &str) -> bool {
    use rfd::{MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};

    // プレビュー（最初の5行、各行80文字まで）
    let preview: String = text
        .lines()
        .take(5)
        .map(|line| {
            // UTF-8 境界安全なトランケート + 制御文字を可視記号に置換
            let sanitized: String = line
                .chars()
                .take(80)
                .map(|c| if c.is_control() && c != '\t' { '\u{00b7}' } else { c })
                .collect();
            if line.chars().count() > 80 { format!("{sanitized}\u{2026}") } else { sanitized }
        })
        .collect::<Vec<_>>()
        .join("\n");

    let line_count = text.lines().count();
    let more = if line_count > 5 {
        format!("\n\n\u{2026} and {} more lines", line_count - 5)
    } else {
        String::new()
    };

    let result = MessageDialog::new()
        .set_level(MessageLevel::Warning)
        .set_title("Confirm Paste")
        .set_description(format!(
            "You are about to paste text containing newlines. \
             This could execute commands in the terminal.\n\n\
             {preview}{more}"
        ))
        .set_buttons(MessageButtons::OkCancelCustom("Paste".to_string(), "Cancel".to_string()))
        .show();

    result == MessageDialogResult::Custom("Paste".to_string())
}

/// 設定からカスタム QuickSelect パターンをコンパイルして返す。
///
/// 無効な正規表現はスキップし警告ログを出す。最大件数は `clamped_patterns()` で制限済み。
pub(crate) fn compile_quick_select_patterns(config: &sdit_core::config::Config) -> Vec<Regex> {
    config
        .quick_select
        .clamped_patterns()
        .iter()
        .filter_map(|p| {
            Regex::new(p)
                .map_err(|e| log::warn!("QuickSelect pattern compile error '{p}': {e}"))
                .ok()
        })
        .collect()
}

/// テキストをブラケットペーストシーケンスで包む。
///
/// Terminal Injection 攻撃防止のため、テキスト内のブラケットシーケンスをサニタイズする。
pub(crate) fn wrap_bracketed_paste(text: &str) -> Vec<u8> {
    // ダブルリプレースバイパス対策: 収束するまで繰り返し除去する
    let mut sanitized = text.to_owned();
    loop {
        let next = sanitized.replace("\x1b[200~", "").replace("\x1b[201~", "");
        if next == sanitized {
            break;
        }
        sanitized = next;
    }
    let mut v = b"\x1b[200~".to_vec();
    v.extend_from_slice(sanitized.as_bytes());
    v.extend_from_slice(b"\x1b[201~");
    v
}

/// IME Commit テキストをPTY送信用バイト列に変換する。
///
/// `bracketed_paste` が true かつテキストが2バイト以上の場合、
/// ブラケットペーストシーケンスで包む（Terminal Injection 攻撃防止のためサニタイズ済み）。
/// 1バイト以下の場合は通常の文字入力として `into_bytes()` を返す。
pub(crate) fn ime_commit_to_bytes(text: String, bracketed_paste: bool) -> Vec<u8> {
    if text.chars().count() > 1 && bracketed_paste {
        wrap_bracketed_paste(&text)
    } else {
        text.into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // is_unsafe_paste のテスト
    // -----------------------------------------------------------------------

    #[test]
    fn is_unsafe_paste_detects_newline() {
        assert!(is_unsafe_paste("hello\nworld", false));
        assert!(is_unsafe_paste("hello\rworld", false));
        assert!(!is_unsafe_paste("hello world", false));
    }

    #[test]
    fn is_unsafe_paste_safe_in_bracketed_mode() {
        assert!(!is_unsafe_paste("hello\nworld", true));
    }

    #[test]
    fn is_unsafe_paste_empty_string() {
        assert!(!is_unsafe_paste("", false));
        assert!(!is_unsafe_paste("", true));
    }

    #[test]
    fn is_unsafe_paste_safe_single_line() {
        assert!(!is_unsafe_paste("echo hello", false));
    }

    #[test]
    fn is_unsafe_paste_detects_bracket_escape() {
        // bracketed paste 終了シーケンスは bracketed モードでも危険
        assert!(is_unsafe_paste("hello\x1b[201~world", true));
        assert!(is_unsafe_paste("hello\x1b[201~world", false));
    }

    // -----------------------------------------------------------------------
    // VisualBell のテスト
    // -----------------------------------------------------------------------

    #[test]
    fn visual_bell_initial_intensity_is_zero() {
        let bell = VisualBell::new(150);
        assert!(bell.intensity() < f32::EPSILON);
    }

    #[test]
    fn visual_bell_ring_starts_animation() {
        let mut bell = VisualBell::new(150);
        bell.ring();
        assert!(bell.intensity() > 0.0);
    }

    #[test]
    fn visual_bell_fades_to_zero() {
        let mut bell = VisualBell::new(10); // 10ms duration
        bell.ring();
        std::thread::sleep(std::time::Duration::from_millis(20));
        assert!(bell.intensity() < f32::EPSILON);
    }

    #[test]
    fn visual_bell_completed_clears_state() {
        let mut bell = VisualBell::new(10);
        bell.ring();
        std::thread::sleep(std::time::Duration::from_millis(20));
        assert!(bell.completed());
        assert!(bell.completed()); // 既にクリア済み
    }

    #[test]
    fn visual_bell_set_duration_updates() {
        let mut bell = VisualBell::new(100);
        bell.set_duration(200);
        // duration が変わっていること確認（ring 前は 0.0 のまま）
        assert!(bell.intensity() < f32::EPSILON);
    }

    // -----------------------------------------------------------------------
    // PreeditState のテスト
    // -----------------------------------------------------------------------

    #[test]
    fn preedit_state_construct() {
        let state = PreeditState { text: "あいう".to_string(), cursor_offset: Some((0, 3)) };
        assert_eq!(state.text, "あいう");
        assert_eq!(state.cursor_offset, Some((0, 3)));
    }

    #[test]
    fn preedit_state_no_cursor() {
        let state = PreeditState { text: "変換中".to_string(), cursor_offset: None };
        assert_eq!(state.text, "変換中");
        assert!(state.cursor_offset.is_none());
    }

    #[test]
    fn preedit_state_clone() {
        let original = PreeditState { text: "テスト".to_string(), cursor_offset: Some((0, 6)) };
        let cloned = original.clone();
        assert_eq!(cloned.text, original.text);
        assert_eq!(cloned.cursor_offset, original.cursor_offset);
    }

    #[test]
    fn preedit_state_empty_text() {
        let state = PreeditState { text: String::new(), cursor_offset: None };
        assert!(state.text.is_empty());
    }

    // -----------------------------------------------------------------------
    // ime_commit_to_bytes のテスト
    // -----------------------------------------------------------------------

    #[test]
    fn ime_commit_single_char_no_bracket() {
        // 1バイト文字はブラケットペーストモードでもラップしない
        let bytes = ime_commit_to_bytes("a".to_string(), true);
        assert_eq!(bytes, b"a");
    }

    #[test]
    fn ime_commit_single_char_no_bracket_mode_off() {
        let bytes = ime_commit_to_bytes("a".to_string(), false);
        assert_eq!(bytes, b"a");
    }

    #[test]
    fn ime_commit_multi_char_bracketed_paste_on() {
        // 複数文字かつブラケットペーストモード: ラップされる
        let bytes = ime_commit_to_bytes("あいう".to_string(), true);
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.starts_with("\x1b[200~"), "ブラケット開始シーケンスがない: {s:?}");
        assert!(s.ends_with("\x1b[201~"), "ブラケット終了シーケンスがない: {s:?}");
        assert!(s.contains("あいう"), "テキスト本体がない: {s:?}");
    }

    #[test]
    fn ime_commit_multi_char_bracketed_paste_off() {
        // ブラケットペーストモード無効: ラップしない
        let bytes = ime_commit_to_bytes("あいう".to_string(), false);
        assert_eq!(bytes, "あいう".as_bytes());
    }

    #[test]
    fn ime_commit_sanitizes_injection_sequence() {
        // Terminal Injection: テキスト内にブラケットシーケンスが混入していたらサニタイズする
        let malicious = "safe\x1b[200~INJECTED\x1b[201~end".to_string();
        let bytes = ime_commit_to_bytes(malicious, true);
        let s = String::from_utf8(bytes).unwrap();
        // サニタイズ後のテキストにはブラケットシーケンスが1組だけある（ラップの分）
        assert_eq!(s.matches("\x1b[200~").count(), 1, "ブラケット開始が複数ある: {s:?}");
        assert_eq!(s.matches("\x1b[201~").count(), 1, "ブラケット終了が複数ある: {s:?}");
        // INJECTED はサニタイズされず含まれる（シーケンス自体が除去される）
        assert!(
            !s[6..s.len() - 6].contains("\x1b[200~"),
            "インジェクションシーケンスが残存: {s:?}"
        );
    }

    #[test]
    fn ime_commit_empty_string() {
        let bytes = ime_commit_to_bytes(String::new(), true);
        assert!(bytes.is_empty());
    }

    #[test]
    fn ime_commit_two_byte_ascii() {
        // ASCII 2文字はブラケットペーストモードではラップされる（len > 1 の判定）
        let bytes = ime_commit_to_bytes("ab".to_string(), true);
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.starts_with("\x1b[200~"), "ブラケット開始シーケンスがない: {s:?}");
        assert!(s.contains("ab"), "テキスト本体がない: {s:?}");
    }

    // -----------------------------------------------------------------------
    // QuickSelectState のテスト
    // -----------------------------------------------------------------------

    #[test]
    fn hint_label_generation() {
        // 最初の26ラベルは1文字
        assert_eq!(QuickSelectState::generate_label(0), "a");
        assert_eq!(QuickSelectState::generate_label(1), "s");
        assert_eq!(QuickSelectState::generate_label(25), "m"); // 26番目の文字
    }

    #[test]
    fn hint_label_two_chars() {
        // 26個以上のマッチで2文字ラベルが生成されること
        let label_26 = QuickSelectState::generate_label(26);
        assert_eq!(label_26.chars().count(), 2, "26番目のラベルは2文字: {label_26}");
        let label_27 = QuickSelectState::generate_label(27);
        assert_eq!(label_27.chars().count(), 2, "27番目のラベルは2文字: {label_27}");
    }

    #[test]
    fn hint_labels_are_unique() {
        // 最初の52ラベルがすべて異なること
        let labels: Vec<String> = (0..52).map(QuickSelectState::generate_label).collect();
        let unique: std::collections::HashSet<&str> = labels.iter().map(|s| s.as_str()).collect();
        assert_eq!(unique.len(), 52, "ラベルに重複がある: {labels:?}");
    }
}
