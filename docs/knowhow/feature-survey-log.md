# 機能発見調査ログ

ブートシーケンスの機能発見フローで調査済みの領域を記録する。
同じ領域を再調査しないために参照する。

## 調査済み領域

### 2026-03-12: 当たり前品質分析（Phase 5.5〜12 ロードマップ策定）

**調査対象リファレンス:**
- Alacritty: `alacritty_terminal/src/ansi.rs`（Handler trait全メソッド）、`alacritty_terminal/src/term/mod.rs`（TermMode フラグ一覧）、`alacritty/src/input/mod.rs`（キー入力・マウス・IME）、`alacritty/src/display/`（カーソル描画・選択表示）
- Ghostty: `src/terminal/modes.zig`（モード一覧）、`src/terminal/cursor.zig`（カーソルスタイル）
- WezTerm: `config/src/bell.rs`（ベル設定）

**発見した機能ギャップ（Plan に反映済み）:**
- DA1/DA2/DSR/CPR レポート → Phase 5.5.1
- DECSCUSR カーソルスタイル → Phase 5.5.1
- Alt→ESC prefix → Phase 5.5.2
- ベル通知 → Phase 5.5.2
- ウィンドウタイトル反映 → Phase 5.5.2
- マウスイベント報告（click/drag/motion/SGR） → Phase 6.1
- テキスト選択 + クリップボード → Phase 6.2
- IME入力 → Phase 7
- フォントサイズ動的変更 → Phase 8.1
- URL検出 → Phase 8.2
- スクロールバック検索 → Phase 9.1
- キーバインドカスタマイズ → Phase 9.2
- 設定Hot Reload → Phase 10.1
- リガチャ/カラー絵文字 → Phase 10.3
- macOSメニューバー → Phase 11.1
- 右クリックメニュー → Phase 11.2
- GUI設定画面 → Phase 11.3

### 2026-03-14: 当たり前品質分析（Phase 13 策定）

**調査対象リファレンス:**
- Alacritty: `alacritty/src/renderer/`（レンダリング最適化）、`alacritty/src/config/window.rs`（ウィンドウ設定）、`alacritty/src/display/bell.rs`（ベル表示）、`alacritty/src/config/cursor.rs`（カーソル設定）
- Ghostty: `src/input/`（入力処理・Kitty keyboard protocol・キーエンコード）、`src/input/paste.zig`（ペースト安全性）、`src/termio/shell_integration.zig`（シェル統合）、`src/apprt/action.zig`（通知）
- WezTerm: `wezterm-gui/src/overlay/`（quickselect・copymode）

**発見した機能ギャップ（Phase 13 Plan に反映済み）:**
- macOS Option as Alt → Phase 13.1
- ビジュアルベル + Dock バウンス → Phase 13.2
- 背景透過 + macOS blur → Phase 13.3
- Unsafe paste 警告 → Phase 13.4
- Kitty Keyboard Protocol → Phase 13.5
- デスクトップ通知 (OSC 9/99) → Phase 13.6

**将来検討（Plan 未作成 → Phase 14 で一部反映）:**
- ~~Quick Select（WezTerm quickselect）~~ → Phase 14.6 に反映
- コピーモード（vi-mode）— キーボードのみでスクロールバック内を選択
- ~~シェルインテグレーション（OSC 133）~~ → Phase 14.5 に反映
- カスタムリンク正規表現 — URL 以外のクリック可能パターン

**未調査領域（次回の候補）:**
- Alacritty: `alacritty/src/config/bindings.rs`（キーバインド設定の詳細構造）
- Ghostty: `src/font/`（フォントシェーピング詳細）、`src/renderer/`（GPU描画最適化）
- WezTerm: `wezterm-mux/src/`（Mux層の詳細）、`wezterm-client/src/`（クライアント/サーバー分離）
- Zellij: `default-plugins/`（プラグインUI設計）、`zellij-server/src/`（セッションサーバー詳細）

### 2026-03-14: 設定精緻化分析（Phase 14 策定）

**調査対象リファレンス:**
- Alacritty: `alacritty/src/config/cursor.rs`（カーソル設定）、`alacritty/src/config/window.rs`（ウィンドウサイズ・パディング）、`alacritty/src/display/mod.rs`（SizeInfo/パディング実装）
- Ghostty: `src/terminal/cursor.zig`（カーソル管理）、`src/termio/shell_integration.zig`（OSC 133）、`src/config/`（設定項目全体）
- WezTerm: `wezterm-gui/src/overlay/quickselect.rs`（QuickSelect）、`config/src/`（スクロールバック・ウィンドウ設定）

**発見した機能ギャップ（Phase 14 Plan に反映済み）:**
- カーソルスタイル・点滅・色の設定 → Phase 14.1
- スクロールバック行数の設定化 → Phase 14.2
- ウィンドウパディング → Phase 14.3
- 初期ウィンドウサイズ指定 → Phase 14.4
- シェルインテグレーション（OSC 133） → Phase 14.5
- Quick Select → Phase 14.6

**将来検討（Plan 未作成）:**
- カスタムシェーダー（shadertoy 互換） — Phase 16.x で検討

### 2026-03-14: Phase 15 策定（入力・フォント・リンク高度化）

**調査対象リファレンス:**
- Alacritty: `alacritty/src/config/bindings.rs`（ViAction/ViMotion）、`alacritty/src/config/scrolling.rs`、`alacritty/src/config/mouse.rs`、`alacritty/src/config/selection.rs`、`alacritty/src/config/font.rs`
- Ghostty: `src/font/Collection.zig`（フォールバック）、`src/font/CodepointMap.zig`、`src/font/Metrics.zig`、`src/renderer/Metal.zig`（カスタムシェーダー）、`src/config/Config.zig`（selection-word-chars, font-variation, link 等）
- WezTerm: `mux/src/`（tab.rs の set_title）
- Zellij: `default-plugins/tab-bar/`、`zellij-utils/src/data.rs`（RenameTab）

**発見した機能ギャップ（Phase 15 Plan に反映済み）:**
- vi モード（コピーモード） → Phase 15.1
- スクロール倍率設定 → Phase 15.2
- セマンティック単語境界文字 → Phase 15.2
- タイピング中マウスカーソル非表示 → Phase 15.2
- 選択時クリップボード自動コピー → Phase 15.2
- フォントフォールバックチェーン → Phase 15.3
- font-codepoint-map → Phase 15.3
- font-variation / font-feature → Phase 15.3
- セル幅・高さ・ベースラインの調整 → Phase 15.3
- カスタムリンク正規表現 → Phase 15.4
- セッションリネーム → Phase 15.4
- フォントバリエーション軸設定 → Phase 15.3

**将来検討（Plan 未作成）:**
- カスタムシェーダー（shadertoy 互換）— Phase 16.x で検討

**未調査領域（次回の候補）:**
- WezTerm: `wezterm-client/src/`（クライアント/サーバー分離の詳細）
- Zellij: `zellij-server/src/`（セッション永続化の詳細）

### 2026-03-14: Phase 16 策定（品質・UX・セキュリティ向上）

**調査対象リファレンス:**
- Ghostty: `src/config/Config.zig`（全体 — scrollbar, maximize, fullscreen, confirm-close, kitty graphics, selection colors, quick-terminal, command-palette, custom-shader, font-thicken, key-remap, secure-input, minimum-contrast, window-inherit-working-directory, scroll-to-bottom, clipboard-trim-trailing-spaces）
- Alacritty: `alacritty/src/config/window.rs`（startup_mode）
- WezTerm: `wezterm-gui/src/scrollbar.rs`、`wezterm-gui/src/overlay/confirm_close_pane.rs`、`wezterm-gui/src/termwindow/palette.rs`

**発見した機能ギャップ（Phase 16 Plan に反映予定）:**
- スクロールバー → Phase 16.1
- 起動モード設定（Maximized/Fullscreen） → Phase 16.2
- 閉じる前の確認ダイアログ → Phase 16.3
- 選択色設定 → Phase 16.4
- クリップボードコピー時末尾空白削除 → Phase 16.5
- Working Directory 継承 → Phase 16.6
- スクロールトゥボトム設定 → Phase 16.7
- 最小コントラスト比 → Phase 16.8
- Secure Keyboard Entry（macOS） → Phase 16.9

**将来検討（Phase 17+ で検討）:**
- Kitty グラフィクスプロトコル — 実装規模大、独立フェーズで計画
- Quick Terminal — macOS 固有UI、独立フェーズで計画
- コマンドパレット — 独立フェーズで計画
- カスタムシェーダー — wgpu post-process パス、独立フェーズで計画
- フォントレンダリング調整（font-thicken） — 独立フェーズで計画
- キーリマップ — 独立フェーズで計画

**未調査領域（次回の候補）:**
- Ghostty: `src/terminal/kitty/`（Kitty グラフィクスプロトコル詳細）
- Ghostty: `src/renderer/`（GPU 描画最適化、シェーダー実装）
- WezTerm: `wezterm-client/src/`（クライアント/サーバー分離の詳細）
- Zellij: `zellij-server/src/`（セッション永続化の詳細）

### 2026-03-14: Phase 17 策定（品質・UX 向上 第2弾）

**調査対象リファレンス:**
- Ghostty: `src/apprt/action.zig`（全アクション列挙 — toggle_window_decorations, float_window, toggle_command_palette, key_sequence 等）
- Ghostty: `src/config/Config.zig`（quick-terminal, font-thicken, right-click, key-remap, theme 等）
- Ghostty: `src/terminal/kitty/`（Kitty グラフィクスプロトコル概要把握）
- Ghostty: `src/renderer/shadertoy.zig`（カスタムシェーダー概要）
- WezTerm: `wezterm-gui/src/termwindow/palette.rs`（コマンドパレット設計）
- WezTerm: `wezterm-client/src/`（クライアント/サーバー分離概要）
- Zellij: `zellij-server/src/session_layout_metadata.rs`（セッション永続化概要）
- Alacritty: `alacritty/src/config/window.rs`（Decorations enum）

**発見した機能ギャップ（Phase 17 Plan に反映済み）:**
- テーマプリセットシステム → Phase 17.1
- ウィンドウデコレーション設定 → Phase 17.2
- Always On Top → Phase 17.3
- 右クリック動作カスタマイズ → Phase 17.4
- コマンドパレット → Phase 17.5
- セッション復帰（タブ・CWD） → Phase 17.6

**将来検討（Phase 18+ で検討）:**
- Kitty グラフィクスプロトコル — 実装規模大、独立フェーズで計画
- Quick Terminal（macOS） — macOS 固有 UI、独立フェーズで計画
- カスタムシェーダー（Shadertoy） — wgpu post-process パス、独立フェーズで計画
- フォントレンダリング調整（font-thicken） — macOS 固有、独立フェーズで計画
- キーリマップ（キーシムレベル変換） — 現行キーバインドでは対応困難
- タブ/ウィンドウ概要表示 — 縦タブバーがあるため優先度低
- インスペクター/デバッグレイヤー — 開発者向けツール

**未調査領域（次回の候補）:**
- Ghostty: `src/terminal/kitty/graphics_exec.zig`（Kitty グラフィクス実行詳細）
- WezTerm: `wezterm-mux/src/`（Mux 層の詳細セッション管理）
- Zellij: `default-plugins/`（プラグイン UI 設計の詳細）

### 2026-03-14: Phase 18 策定（ビジュアル・高度機能）

**調査対象リファレンス:**
- Ghostty: `src/config/Config.zig`（background-image, quick-terminal, font-thicken, clipboard-codepoint-map）
- Ghostty: `src/apprt/action.zig`（float_window, toggle_quick_terminal, inspector）
- WezTerm: `wezterm-gui/src/`（scripting, overlay 再スキャン）
- Alacritty: `alacritty/src/`（再スキャン）

**発見した機能ギャップ（Phase 18 Plan に反映済み）:**
- 背景画像 → Phase 18.1
- Quick Terminal（macOS ドロップダウン） → Phase 18.2
- フォント太さ調整（macOS） → Phase 18.3
- クリップボード文字変換 → Phase 18.4

**将来検討（Phase 20+）:**
- Kitty グラフィクスプロトコル / カスタムシェーダー / Lua スクリプティング / デバッグインスペクター / キーリマップ

### 2026-03-14: Phase 19 策定（品質・カスタマイズ向上）

**調査対象リファレンス:**
- Ghostty: `src/config/Config.zig`（全設定項目再スキャン — env, working-directory, click-repeat-interval, grapheme-width-method, search-foreground/background, window-padding-color, window-subtitle, notify-on-command-finish）
- Alacritty: `alacritty/src/config/`（ウィンドウ・マウス・スクロール・選択・端末設定）
- WezTerm: `config/src/config.rs`（フォント・レンダリング・ウィンドウ設定）

**既に実装済みと判明した機能（除外）:**
- セルメトリクス調整（adjust-cell-width/height/baseline）→ Phase 15.3 で実装済み
- font-variation / font-feature → Phase 15.3 で実装済み
- font-codepoint-map → Phase 15.3 で実装済み
- font-thicken → Phase 18.3 で実装済み
- copy-on-select → save_to_clipboard として Phase 15.2 で実装済み

**発見した機能ギャップ（Phase 19 Plan に反映済み）:**
- 環境変数注入 + 初期ワーキングディレクトリ → Phase 19.1
- 検索ハイライト色 + パディング背景色 → Phase 19.2
- コマンド終了通知 → Phase 19.3
- ダブルクリック判定間隔 + Grapheme 幅方式 → Phase 19.4
- ウィンドウサブタイトル → Phase 19.5

**将来検討（Phase 20+）:**
- Kitty グラフィクスプロトコル — 実装規模大
- カスタムシェーダー（Shadertoy 互換）— wgpu post-process パス
- キーリマップ — キーシムレベル変換
- デバッグインスペクター — 開発者向けツール

### 2026-03-14: Phase 20 策定（品質・カスタマイズ向上 第2弾）

**調査対象リファレンス:**
- Ghostty: `src/config/Config.zig`（全設定項目再スキャン — bold-is-bright, faint-opacity, window-position, focus-follows-mouse, osc-color-report-format, title-report, enquiry-response, palette-generate, alpha-blending 等）
- WezTerm: `config/src/config.rs`（font_rasterizer, font_locator 詳細設定）
- Alacritty: `alacritty/src/config/`（再スキャン）

**発見した機能ギャップ（Phase 20 Plan に反映済み）:**
- Bold as Bright + Faint Opacity → Phase 20.1
- ウィンドウ座標保存 → Phase 20.2
- Focus Follows Mouse → Phase 20.3
- OSC Color Report Format → Phase 20.4
- Title Report Flag → Phase 20.5
- Enquiry Response → Phase 20.6

**将来検討（Phase 21+）:**
- Kitty グラフィクスプロトコル — 実装規模大
- カスタムシェーダー（Shadertoy 互換）— wgpu post-process パス
- Palette Generation / Harmonious — 配色自動生成
- Alpha Blending Mode — sRGB/Linear 切り替え
- Font Locator/Rasterizer 詳細設定 — FreeType/CoreText hinting 等

**未調査領域（次回の候補）:**
- Ghostty: `src/terminal/kitty/graphics_exec.zig`（Kitty グラフィクス実行詳細）
- Ghostty: `src/renderer/shadertoy.zig`（カスタムシェーダー実装詳細）
- WezTerm: `wezterm-mux/src/`（Mux 層の詳細セッション管理）

### 2026-03-15: Phase 25 策定（当たり前品質・オプション機能）

**調査対象リファレンス:**
- WezTerm: `refs/wezterm/config/src/config.rs`（全量スキャン — resize_increments, treat_east_asian_ambiguous_width_as_wide, swallow_mouse_click_on_window_focus 等）
- Ghostty: `refs/ghostty/src/config/Config.zig`（全体概観 — window-colorspace, macos-dock-drop-behavior, macos-window-buttons 等）

**発見した機能ギャップ（Phase 25 Plan に反映済み）:**
- ウィンドウリサイズのセル整数倍スナップ → Phase 25.1
- 東アジア曖昧幅文字の広幅扱い設定 → Phase 25.2
- ウィンドウ色空間選択（macOS Display P3） → Phase 25.3
- ウィンドウフォーカス時マウスクリック抑制 → Phase 25.4

**将来検討（Plan 未作成）:**
- Kitty グラフィクスプロトコル — 実装規模大、独立フェーズで計画
- カスタムシェーダー（Shadertoy）— wgpu post-process パス
- Bidi テキストサポート — アラビア語・ヘブライ語
- macOS Dock ドロップ挙動設定 (`macos-dock-drop-behavior`)
- macOS ウィンドウボタン表示制御 (`macos-window-buttons`)
