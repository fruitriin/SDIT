# テストシナリオ インデックス

| シナリオ | 要約 | 最終実行 | 結果 |
|---|---|---|---|
| [001-basic-echo](001-basic-echo.md) | 基本的な echo 動作確認 | 2026-03-15 | PASS |
| [002-window-resize](002-window-resize.md) | ウィンドウリサイズ時のグリッド再計算確認 | 2026-03-13 | PARTIAL |
| [003-multi-window](003-multi-window.md) | 複数ウィンドウ（Cmd+N）の生成・独立動作確認 | 2026-03-15 | PASS |
| [004-window-independent-close](004-window-independent-close.md) | ウィンドウ独立クローズの確認 | 2026-03-13 | PASS |
| [005-session-add-sidebar](005-session-add-sidebar.md) | セッション追加とサイドバー表示確認 | 2026-03-13 | PASS |
| [006-session-switch](006-session-switch.md) | セッション切り替え（サイドバークリック）確認 | - | - |
| [007-session-detach](007-session-detach.md) | セッション切出し（ドラッグアウト）確認 | - | - |
| [008-config-font-theme](008-config-font-theme.md) | 設定ファイルによるフォント・テーマ変更確認 | - | - |
| [009-cjk-display](009-cjk-display.md) | CJK 全角文字の表示確認 | 2026-03-14 | PARTIAL |
| [010-alt-key](010-alt-key.md) | Alt キー入力の確認 | - | - |
| [011-window-title](011-window-title.md) | ウィンドウタイトル動的更新の確認 | - | - |
| [012-cursor-style](012-cursor-style.md) | DECSCUSR カーソルスタイル変更 + DECSCUSR 0 デフォルト復帰 + 設定ファイルデフォルト確認 | 2026-03-14 | UPDATED |
| [013-ime-input](013-ime-input.md) | IME 入力（日本語）の確認 | 2026-03-13 | UNIT_ONLY |
| [014-font-size-change](014-font-size-change.md) | フォントサイズ動的変更（Cmd+=/Cmd+-/Cmd+0）の確認 | 2026-03-13 | PASS |
| [015-url-detection](015-url-detection.md) | URL 検出・Cmd+クリック・ホバーハイライトの確認 | 2026-03-13 | UNIT_ONLY |
| [016-scrollback-search](016-scrollback-search.md) | スクロールバック内検索（Cmd+F）の確認 | - | - |
| [017-keybind-customization](017-keybind-customization.md) | デフォルト・カスタム・不正値フォールバック・プラットフォーム固有キーバインドの確認 | 2026-03-13 | PENDING |
| [018-config-hot-reload](018-config-hot-reload.md) | 設定ファイル変更の動的反映（フォントサイズ・カラーテーマ・キーバインド）とエラー時 graceful fallback の確認 | - | - |
| [019-window-persistence](019-window-persistence.md) | ウィンドウサイズ・位置の永続化（session.toml への保存と起動時復元）の確認 | 2026-03-13 | UNIT_ONLY |
| [020-color-emoji](020-color-emoji.md) | Atlas RGBA 化とカラー絵文字描画（SwashContent 種別ごとの変換・is_color フラグ）の確認 | 2026-03-13 | PASS |
| [021-context-menu](021-context-menu.md) | 右クリックコンテキストメニュー（ターミナル領域: Copy/Paste/Select All/Search、サイドバー領域: Close Session/Move to New Window）の表示・動作確認 | 2026-03-13 | UNIT_ONLY |
| [022-config-serialize-template](022-config-serialize-template.md) | Config Serialize + TOML テンプレート生成（save/save_with_comments/Preferences ハンドラ）の確認 | 2026-03-13 | UNIT_ONLY |
| [023-opentype-ligature](023-opentype-ligature.md) | OpenType リガチャ（`->`, `=>` 等）の検出・シェーピング・複数セル幅描画確認 | 2026-03-13 | UNIT_ONLY |
| [024-window-padding](024-window-padding.md) | ウィンドウパディング（padding_x/padding_y）設定・描画オフセット・Hot Reload の確認 | 2026-03-14 | UNIT_ONLY |
| [025-shell-integration-osc133](025-shell-integration-osc133.md) | OSC 133 A/B/C/D セマンティックゾーンマーカー記録・Cmd+Up/Down プロンプトジャンプ・ShellIntegration 有効/無効の確認 | 2026-03-14 | UNIT_ONLY |
| [026-quick-select](026-quick-select.md) | Cmd+Shift+Space Quick Select モード・URL/パス/ハッシュ/数値パターン検出・ヒントキーコピー・Escape キャンセル・カスタムパターン設定の確認 | 2026-03-14 | UNIT_ONLY |
| [027-vi-mode](027-vi-mode.md) | vi モード起動/終了（Cmd+Shift+V/Escape）・hjkl 基本移動・v 文字選択・V 行選択・y ヤンク・/ 検索連携・ブロックカーソル描画の確認 | 2026-03-14 | UNIT_ONLY |
| [028-macos-menubar](028-macos-menubar.md) | macOS ネイティブメニューバー（sdit/File/Edit/View/Session）の構造確認・メニュー操作の動作確認・Phase 21.6 クラッシュ修正確認 | 2026-03-15 | PASS |
| [029-image-tools](029-image-tools.md) | annotate-grid（--divide/--every/スタイルオプション）と clip-image（--grid-cell/--rect）の動作確認・エッジケースバリデーション | 2026-03-14 | PARTIAL |
| [030-pty-deadlock-fix](030-pty-deadlock-fix.md) | PTY デッドロック修正確認（DA/DSR クエリ後の停滞回避・メニュー操作との組み合わせ） | 2026-03-15 | PASS |
| [031-tab-drag-detach](031-tab-drag-detach.md) | タブドラッグ切り出し（Chrome-like UX）・メニュー操作による DetachSession 代替確認 | 2026-03-15 | UNIT_ONLY |
