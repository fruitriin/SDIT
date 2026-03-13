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
- コピーモード（vi-mode）— キーボードのみでスクロールバック内を選択
- カスタムリンク正規表現 — URL 以外のクリック可能パターン
- フォントフォールバックチェーン — 複数フォントの優先順位指定
- セル間隔の微調整 — letter-spacing 相当の設定
