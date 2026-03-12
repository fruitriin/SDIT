# テストシナリオ INDEX

> このファイルは手動・エージェントにより更新される。シナリオ追加・実行時は必ず更新すること。

## フォーマット

| シナリオ | 要約 | 最終実行 | 結果 |
|---|---|---|---|
| ファイル名 | 何をテストするか | YYYY-MM-DD or 未実行 | ok / FAIL / — |

---

## シナリオ一覧

| シナリオ | 要約 | 最終実行 | 結果 |
|---|---|---|---|
| [001-basic-echo.md](001-basic-echo.md) | キー入力が PTY に届き echo 結果がウィンドウに描画される | 未実行 | — |
| [002-window-resize.md](002-window-resize.md) | ウィンドウリサイズで PTY が連動してリサイズされる | 未実行 | — |
| [003-multi-window.md](003-multi-window.md) | Cmd+N で複数ウィンドウを生成・独立動作する | 未実行 | — |
| [004-window-independent-close.md](004-window-independent-close.md) | Cmd+W でウィンドウを個別に閉じられる | 未実行 | — |
| [005-session-add-sidebar.md](005-session-add-sidebar.md) | セッション追加でサイドバーが自動出現する | 未実行 | — |
| [006-session-switch.md](006-session-switch.md) | サイドバーでセッションを切り替えられる | 未実行 | — |
| [007-session-detach.md](007-session-detach.md) | セッションをドラッグアウトして独立ウィンドウに切り出せる | 未実行 | — |
| [008-config-font-theme.md](008-config-font-theme.md) | TOML 設定のフォント・カラーテーマが起動時に反映される | 未実行 | — |
| [009-cjk-display.md](009-cjk-display.md) | 日本語 CJK 文字が豆腐にならず全角幅で描画される | 未実行 | — |
| [010-alt-key.md](010-alt-key.md) | Alt+key が ESC プレフィックス付きで PTY に送信される | 未実行 | — |
| [011-window-title.md](011-window-title.md) | OSC 2 エスケープシーケンスでウィンドウタイトルが変わる | 未実行 | — |
| [012-cursor-style.md](012-cursor-style.md) | DECSCUSR でカーソルスタイルが変わる | 未実行 | — |
| [013-ime-input.md](013-ime-input.md) | macOS IME（日本語入力）のプリエディット・確定が動作する | 未実行 | — |
| [014-font-size-change.md](014-font-size-change.md) | Cmd+=/Cmd+-/Cmd+0 でフォントサイズが変わり、連続操作でクラッシュしない | 2026-03-13 | ユニットテストのみ実行（ディスプレイスリープのため GUI 不可） |

---

## 実行状況サマリー

| 実行日 | 対象シナリオ | 実行環境 | 結果 |
|---|---|---|---|
| 2026-03-13 | 014 (ユニットテスト相当) | macOS（ディスプレイスリープ中） | `cargo test set_font_size` → 3件 ok |

---

## 注意事項

- GUI テストはディスプレイがアクティブな状態でのみ実行可能
- `Display Asleep: Yes` の場合、screencapture が真っ黒な画像を返す
- capture-window の screencapture フォールバックはウィンドウ座標が (0,0) サイズの場合に失敗する
- headless / CI 環境では GUI テストをスキップし `#[ignore]` テストを明示的に除外すること
