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
use sdit_core::terminal::url_detector::UrlDetector;

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
    /// 設定ファイルから読み込んだデフォルトフォントサイズ（Cmd+0 でこのサイズに復帰）。
    pub(crate) default_font_size: f32,
    /// URL 検出器（正規表現コンパイル済み）。
    pub(crate) url_detector: UrlDetector,
    /// 現在 URL ホバー中の状態（Cmd/Ctrl 押下中に URL 上にカーソルがある場合）。
    pub(crate) hovered_url: Option<UrlHoverState>,
    /// 検索バーの状態。None = 検索バー非表示。
    pub(crate) search: Option<SearchState>,
    /// 設定全体（キーバインド等）。
    pub(crate) config: sdit_core::config::Config,
}

impl SditApp {
    pub(crate) fn new(
        event_proxy: winit::event_loop::EventLoopProxy<SditEvent>,
        smoke_test: bool,
        config: &sdit_core::config::Config,
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
            cursor_blink_visible: true,
            cursor_blink_last_toggle: std::time::Instant::now(),
            preedit: None,
            default_font_size,
            url_detector: UrlDetector::new(),
            hovered_url: None,
            search: None,
            config: config.clone(),
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
        let window_ids: Vec<winit::window::WindowId> = self.windows.keys().copied().collect();
        for window_id in window_ids {
            let Some(ws) = self.windows.get(&window_id) else { continue };
            let sidebar_w = ws.sidebar.width_px(metrics.cell_width);
            let surface_w = ws.gpu.surface_config.width as f32;
            let surface_h = ws.gpu.surface_config.height as f32;
            let term_width = (surface_w - sidebar_w).max(0.0);
            let (cols, rows) =
                calc_grid_size(term_width, surface_h, metrics.cell_width, metrics.cell_height);

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
}
