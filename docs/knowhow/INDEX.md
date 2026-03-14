# Knowhow Index

> 自動生成。`/knowhow-index reindex` で再生成できる。

## ターミナルエミュレーション

| ファイル | 要約 | キーワード |
|---|---|---|
| [vte-integration.md](vte-integration.md) | vte 0.13 の Perform trait によるエスケープシーケンス処理の統合パターンとテスト構成 | `vte::Perform`, `csi_dispatch`, `esc_dispatch`, `osc_dispatch`, `input_needs_wrap`, `SGR 38;5;N`, `SGR 38:2:r:g:b`, `Line::as_viewport_idx`, `saturating_sub`, `MAX_TITLE_BYTES`, `scroll_up`, `headless_pipeline` |
| [vte-terminal-integration.md](vte-terminal-integration.md) | vte 0.13 の Params サブパラメータ構造、CSI デフォルト値、Alternate Screen Buffer 実装の実践知見 | `vte::Params`, `ParamsIter`, `first_param`, `nth_param`, `handler.rs`, `erase_cells`, `std::mem::swap`, `inactive_grid`, `cursor_cell`, `IndexMut`, `template.clone`, `cast_possible_wrap`, `i32::try_from` |
| [grid-implementation.md](grid-implementation.md) | Grid/Row/Storage のリングバッファ設計と O(1) スクロール実装の詳細 | `Storage<T>`, `Row<T>`, `Grid<T>`, `zero` offset, `compute_index`, `rotate`, `rezero`, `occ` dirty tracking, `saturating_*`, `Line(i32)`, `Column(usize)`, `scroll_up`, `enforce_scroll_limit`, `max_scroll_limit` |
| [terminal-device-reports.md](terminal-device-reports.md) | DA1/DA2/DSR/CPR デバイスレポート応答と Alt-ESC/カーソルスタイルの実装パターン | `pending_writes`, `drain_pending_writes`, `write_response`, `MAX_PENDING_WRITES`, `DA1`, `DA2`, `intermediates`, `is_private`, `DECSCUSR`, `Alt+ESC prefix`, `set_title` デッドロック回避, `clone` before unlock |

## レンダリング

| ファイル | 要約 | キーワード |
|---|---|---|
| [wgpu-instanced-rendering.md](wgpu-instanced-rendering.md) | wgpu インスタンス描画による全セル一括レンダリングと CellVertex/WGSL シェーダー設計（is_color_glyph フラグ含む） | `CellVertex`, `draw(0..6, 0..cell_count)`, `Instance` step mode, `vertex_index`, `QUAD_UV`, `Rgba8Unorm`, `ALPHA_BLENDING`, `bytemuck::Pod`, `grid_pos`, `uv`, `glyph_offset`, `glyph_size`, `is_color_glyph` |
| [wgpu-winit-integration.md](wgpu-winit-integration.md) | wgpu 0.20 + winit 0.30 の Surface ライフタイム管理、アトラス、GPU バッファ動的リサイズ | `Surface<'static>`, `Arc<Window>`, `GpuContext`, `ApplicationHandler`, `resumed`, `window_event`, `user_event`, `SurfaceError::Lost`, `ensure_capacity`, `shelf algorithm`, `upload_if_dirty`, `SwashCache::get_image_uncached` |
| [cosmic-text-glyph-rasterize.md](cosmic-text-glyph-rasterize.md) | cosmic-text によるグリフラスタライズフローとキャッシュキー設計（SwashContent RGBA 変換含む） | `FontSystem::new`, `Buffer`, `Metrics`, `Shaping::Advanced`, `shape_until_scroll`, `LayoutRun`, `PhysicalGlyph`, `SwashImage`, `Placement`, `monospace_em_width`, `fontdb::ID`, `GlyphCacheKey`, `SwashContent::Mask`, `SwashContent::Color`, `BGRA→RGBA`, `is_color` |
| [cosmic-text-glyph-api.md](cosmic-text-glyph-api.md) | cosmic-text 0.12.1 の LayoutGlyph API 注意点: metadata≠バイトオフセット、set_size() 必須 | `glyph.start`, `glyph.end`, `glyph.metadata`, `Buffer::set_size`, `f32::MAX`, `shape_until_scroll`, `layout_runs` |
| [session-sidebar-rendering.md](session-sidebar-rendering.md) | サイドバーの origin_x レイアウト分割、CellPipeline 再利用、セッション切出しロールバック | `origin_x`, `sidebar_width_px`, `build_sidebar_cells`, `CellPipeline`, `detach_session_to_new_window`, `insert(original_index)`, `CursorMoved`, `MouseInput`, `sessions.swap`, `active_index`, `calc_grid_size` |

## PTY・スレッド管理

| ファイル | 要約 | キーワード |
|---|---|---|
| [pty-threading-model.md](pty-threading-model.md) | 3スレッドモデル（Main/Reader/Writer）、fd クローン分離、シャットダウンシーケンスの設計 | `try_clone_writer`, `try_clone_to_owned`, `dup(2)`, `AsFd`, `OwnedFd`, `sync_channel(64)`, `try_send`, `WouldBlock`, `EIO`, `PoisonError::into_inner`, `SIGHUP`, `SIGKILL`, `child_exited: AtomicBool`, `to_rustix_pid`, `tcsetwinsize`, `APP_CURSOR` |

## アーキテクチャ・設計

| ファイル | 要約 | キーワード |
|---|---|---|
| [architecture-decisions.md](architecture-decisions.md) | 2クレート構成、スレッドモデル、Session/Window 分離、縦タブバーの設計判断まとめ | `sdit-core`, `sdit (bin)`, `Mailbox`, `RwLock`, `display_offset`, `damage tracking`, `Session ≠ Window`, `Surface 差し替え`, `polling vs tokio`, `cosmic-text`, `MAX_CACHE_SIZE`, `1500行再分割基準` |
| [multi-window-session-management.md](multi-window-session-management.md) | Session/Window 分離パターン、spawn_reader クロージャ注入、ChildExit ライフサイクル | `SessionId`, `WindowId`, `SessionManager`, `SpawnParams`, `spawn_reader`, `EventLoopProxy`, `SditEvent::PtyOutput`, `SditEvent::ChildExit`, `session_to_window`, `close_window`, `detach` |

## 設定・テーマ

| ファイル | 要約 | キーワード |
|---|---|---|
| [config-and-theming.md](config-and-theming.md) | TOML 設定基盤、f32 クランプの NaN 対策、WCAG コントラスト比検証、CJK 全角描画 | `#[serde(default)]`, `Config::load`, `dirs::config_dir`, `f32::is_finite`, `ResolvedColors`, `hex_to_rgba`, `WCAG 2.1`, `sRGB 線形化`, `WIDE_CHAR`, `WIDE_CHAR_SPACER`, `cell_width_scale: 2.0`, `atomic rename`, `TOCTOU` |
| [cursor-config.md](cursor-config.md) | カーソル設定（style/blinking/color）の実装: serde/内部型分離、DECSCUSR 0 デフォルト復帰、hex カラー変換、hot reload パターン | `CursorStyleConfig`, `CursorConfig`, `CursorStyle`, `From<CursorStyleConfig>`, `new_with_cursor`, `set_default_cursor`, `default_cursor_style`, `default_cursor_blinking`, `DECSCUSR 0`, `parse_hex_color`, `SessionManager::all`, `terminal/tests.rs`, `#[cfg(test)] mod tests;` |


## macOS GUI 統合

| ファイル | 要約 | キーワード |
|---|---|---|
| [context-menu-macos.md](context-menu-macos.md) | macOS 右クリックコンテキストメニューの muda 統合、MenuEvent 共有マップ、unsafe スコープ限定パターン | `show_context_menu_for_nsview`, `#![allow(unsafe_code)]`, `SharedMenuActions`, `Arc<Mutex<HashMap<MenuId, Action>>>`, `extend()`, `MouseButton::Right`, `dead_code` |

## UI インタラクション

| ファイル | 要約 | キーワード |
|---|---|---|
| [quick-select-overlay.md](quick-select-overlay.md) | Quick Select モードのオーバーレイ実装: パターンマッチ・ヒントラベル生成・CellVertex 上書き描画・キー処理 | `QuickSelectState`, `QuickSelectHint`, `generate_label`, `CHARS homerow`, `detect_patterns_in_line`, `default_quick_select_patterns`, `overwrite_cell`, `Action::QuickSelect`, `Cmd+Shift+Space`, `patterns: Vec<String>` |

## シェルインテグレーション

| ファイル | 要約 | キーワード |
|---|---|---|
| [shell-integration-osc133.md](shell-integration-osc133.md) | OSC 133 パーサー・SemanticMarker VecDeque 設計・プロンプトジャンプのスクロール計算・fish 互換性 | `SemanticZone`, `SemanticMarker`, `VecDeque`, `push_back`, `pop_front`, `MAX_SEMANTIC_MARKERS`, `prev_prompt`, `next_prompt`, `shell_integration_enabled`, `display_offset`, `Scroll::Delta` |

## テスト・品質保証

| ファイル | 要約 | キーワード |
|---|---|---|
| [integration-testing-patterns.md](integration-testing-patterns.md) | 3層テスト構成（ヘッドレス/GUI スモーク/GUI 操作）と macOS 権限モデルの知見 | `--headless`, `SDIT_SMOKE_TEST=1`, `smoke_headless.rs`, `smoke_gui.rs`, `gui_interaction.rs`, `wait_with_timeout`, `try_wait`, `AXUIElement`, `ScreenCaptureKit`, `send-keys.sh`, `osascript`, `Screen Recording 権限` |
| [gui-test-cjk-validation.md](gui-test-cjk-validation.md) | CJK テキスト描画の対照群検証: render-text (CoreText) + verify-text (OCR/輝度/SSIM 3層)。トークン効率設計、セル単位 SSIM、左右非対称クリッピング検出 | `render-text`, `verify-text`, `CoreText`, `Vision.framework OCR`, `SSIM`, `per-cell SSIM`, `輝度分析`, `右端クリッピング`, `--mono`, `--cell-info`, `/render-text`, `/verify-text` |
| [gui-test-ime-interference.md](gui-test-ime-interference.md) | macOS GUI テストで日本語 IME が有効だと AppleScript keystroke が文字化けする問題。英数キー送信またはクリップボード経由で回避 | `keystroke`, `key code 102`, `英数キー`, `pbcopy`, `Cmd+V`, `set_ime_allowed`, `AppleScript`, `IME バイパス`, `クリップボード経由` |
| [gui-test-process-identification.md](gui-test-process-identification.md) | 同名プロセス複数存在時にテストツールが意図しないプロセスを操作する問題と回避策 | `pgrep -x`, `ps -eo pid,comm`, `window-info`, `send-keys.sh`, `capture-window`, `--pid`, `pkill`, `trap cleanup EXIT` |
| [gui-test-screen-recording-permission.md](gui-test-screen-recording-permission.md) | capture-window の CGS_REQUIRE_INIT アサーション失敗・ディスプレイスリープ中の黒画像問題と対処法 | `CGS_REQUIRE_INIT`, `ScreenCaptureKit`, `exit code 134`, `SIGABRT`, `screencapture -R`, `画面収録権限`, `VSCode 再起動`, `Display Asleep`, `黒画像`, `window size 0` |

## 開発プロセス・ツール

| ファイル | 要約 | キーワード |
|---|---|---|
| [delegating-to-codex-copilot.md](delegating-to-codex-copilot.md) | Codex CLI / Copilot CLI へのタスク移譲: モデル特性、プロンプトのコツ、パーミッション変換、振り分け基準 | `codex exec`, `copilot -p`, `--full-auto`, `--allow-tool`, `AGENTS.md`, `GPT-5 Codex`, `GPT-5 mini`, `permissions-to-flags.sh`, `sandbox`, `workspace-write` |

## アーカイブ済み

特定の問題のワークアラウンドで、今後同じ問題に遭遇する可能性が低いもの。

| ファイル | 要約 |
|---|---|
| [archived/macos26-pty-compat.md](archived/macos26-pty-compat.md) | macOS 26 で TIOCSWINSZ ioctl が spawn 前に ENOTTY を返す問題。spawn 後に resize を呼ぶことで回避 |
