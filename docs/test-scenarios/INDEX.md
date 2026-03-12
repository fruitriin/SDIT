# テストシナリオ インデックス

| シナリオ | 要約 | 最終実行 | 結果 |
|---|---|---|---|
| [001-basic-echo](001-basic-echo.md) | 基本的な echo 動作確認 | - | - |
| [002-window-resize](002-window-resize.md) | ウィンドウリサイズ時のグリッド再計算確認 | - | - |
| [003-multi-window](003-multi-window.md) | 複数ウィンドウ（Cmd+N）の生成・独立動作確認 | - | - |
| [004-window-independent-close](004-window-independent-close.md) | ウィンドウ独立クローズの確認 | - | - |
| [005-session-add-sidebar](005-session-add-sidebar.md) | セッション追加とサイドバー表示確認 | - | - |
| [006-session-switch](006-session-switch.md) | セッション切り替え（サイドバークリック）確認 | - | - |
| [007-session-detach](007-session-detach.md) | セッション切出し（ドラッグアウト）確認 | - | - |
| [008-config-font-theme](008-config-font-theme.md) | 設定ファイルによるフォント・テーマ変更確認 | - | - |
| [009-cjk-display](009-cjk-display.md) | CJK 全角文字の表示確認 | - | - |
| [010-alt-key](010-alt-key.md) | Alt キー入力の確認 | - | - |
| [011-window-title](011-window-title.md) | ウィンドウタイトル動的更新の確認 | - | - |
| [012-cursor-style](012-cursor-style.md) | カーソルスタイル変更の確認 | - | - |
| [013-ime-input](013-ime-input.md) | IME 入力（日本語）の確認 | - | - |
| [014-font-size-change](014-font-size-change.md) | フォントサイズ動的変更（Cmd+=/Cmd+-/Cmd+0）の確認 | 2026-03-13 | PASS |
| [015-url-detection](015-url-detection.md) | URL 検出・Cmd+クリック・ホバーハイライトの確認 | 2026-03-13 | UNIT_ONLY |
| [016-scrollback-search](016-scrollback-search.md) | スクロールバック内検索（Cmd+F）の確認 | - | - |
| [017-keybind-customization](017-keybind-customization.md) | デフォルト・カスタム・不正値フォールバック・プラットフォーム固有キーバインドの確認 | 2026-03-13 | PENDING |
| [018-config-hot-reload](018-config-hot-reload.md) | 設定ファイル変更の動的反映（フォントサイズ・カラーテーマ・キーバインド）とエラー時 graceful fallback の確認 | - | - |
| [019-window-persistence](019-window-persistence.md) | ウィンドウサイズ・位置の永続化（session.toml への保存と起動時復元）の確認 | 2026-03-13 | UNIT_ONLY |
| [020-color-emoji](020-color-emoji.md) | Atlas RGBA 化とカラー絵文字描画（SwashContent 種別ごとの変換・is_color フラグ）の確認 | 2026-03-13 | PASS |
