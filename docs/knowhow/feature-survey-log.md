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

**未調査領域（次回の候補）:**
- Alacritty: `alacritty/src/renderer/`（レンダリング最適化手法）、`alacritty/src/config/bindings.rs`（キーバインド設定の詳細構造）
- Ghostty: `src/font/`（フォントシェーピング詳細）、`src/renderer/`（GPU描画最適化）、`src/input/`（入力処理の詳細）
- WezTerm: `wezterm-mux/src/`（Mux層の詳細）、`wezterm-gui/src/overlay/`（検索・コピーモード等のオーバーレイUI）、`wezterm-client/src/`（クライアント/サーバー分離）
- Zellij: `default-plugins/`（プラグインUI設計）、`zellij-server/src/`（セッションサーバー詳細）
