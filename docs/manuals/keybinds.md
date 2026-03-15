# キーボードショートカット

## macOS デフォルト

### ウィンドウ・セッション

| ショートカット | アクション |
|---|---|
| Cmd+N | 新しいウィンドウ |
| Cmd+Shift+N | セッション切り出し（独立ウィンドウ化） |
| Cmd+T | 新しいセッション（タブ追加） |
| Cmd+W | セッション/ウィンドウを閉じる |
| Cmd+\\ | セッションサイドバーのトグル |
| Cmd+Q | アプリケーション終了 |

### セッション切り替え

| ショートカット | アクション |
|---|---|
| Ctrl+Tab | 次のセッション |
| Ctrl+Shift+Tab | 前のセッション |
| Cmd+Shift+] | 次のセッション |
| Cmd+Shift+[ | 前のセッション |

### 編集

| ショートカット | アクション |
|---|---|
| Cmd+C | コピー |
| Cmd+V | ペースト |
| Cmd+A | 全選択 |

### 表示

| ショートカット | アクション |
|---|---|
| Cmd+= | フォント拡大 |
| Cmd+Shift++ | フォント拡大 |
| Cmd+- | フォント縮小 |
| Cmd+0 | フォントサイズリセット |

### 検索・ナビゲーション

| ショートカット | アクション |
|---|---|
| Cmd+F | 検索 |
| Cmd+G | 次の検索結果 |
| Cmd+Shift+G | 前の検索結果 |
| Cmd+↑ | 前のプロンプトへジャンプ |
| Cmd+↓ | 次のプロンプトへジャンプ |

### その他

| ショートカット | アクション |
|---|---|
| Cmd+Shift+Space | QuickSelect モード |
| Cmd+Shift+V | Vi モード（コピーモード） |
| Cmd+, | 設定ファイルを開く |

## Linux/Windows デフォルト

| ショートカット | アクション |
|---|---|
| Ctrl+Shift+N | 新しいウィンドウ |
| Ctrl+Shift+T | 新しいセッション |
| Ctrl+Shift+W | セッション/ウィンドウを閉じる |
| Ctrl+\\ | セッションサイドバーのトグル |
| Ctrl+Shift+C | コピー |
| Ctrl+Shift+V | ペースト |
| Ctrl+F | 検索 |
| Ctrl+G | 次の検索結果 |
| Ctrl+Shift+G | 前の検索結果 |
| Ctrl+Tab | 次のセッション |
| Ctrl+Shift+Tab | 前のセッション |

## カスタムキーバインド

設定ファイルの `[[keybinds]]` セクションでキーバインドを追加・上書きできます。

```toml
[[keybinds]]
key = "n"
mods = "super"
action = "NewWindow"

[[keybinds]]
key = "j"
mods = "ctrl|shift"
action = "NextSession"
```

### キーの指定

- 1文字キー: `"a"`, `"z"`, `"1"` など（大文字小文字は区別しない）
- 特殊キー: `Tab`, `Enter`, `Backspace`, `Escape`, `Space`
- 方向キー: `Up`, `Down`, `Left`, `Right`
- ページ: `PageUp`, `PageDown`, `Home`, `End`
- エイリアス: `backslash` → `\`, `plus` → `+`

### モディファイアの指定

`|` で複合できます。

| モディファイア | エイリアス |
|---|---|
| `super` | `cmd`, `logo` |
| `ctrl` | `control` |
| `shift` | — |
| `alt` | `option` |

例: `"super|shift"`, `"ctrl|alt"`

### 使用可能なアクション

| アクション | 説明 |
|---|---|
| `NewWindow` | 新しいウィンドウ |
| `AddSession` | 新しいセッション |
| `CloseSession` | セッション/ウィンドウを閉じる |
| `DetachSession` | セッション切り出し |
| `SidebarToggle` | サイドバーのトグル |
| `Copy` | コピー |
| `Paste` | ペースト |
| `SelectAll` | 全選択 |
| `ZoomIn` | フォント拡大 |
| `ZoomOut` | フォント縮小 |
| `ZoomReset` | フォントサイズリセット |
| `Search` | 検索を開く |
| `SearchNext` | 次の検索結果 |
| `SearchPrev` | 前の検索結果 |
| `NextSession` | 次のセッション |
| `PrevSession` | 前のセッション |
| `PrevPrompt` | 前のプロンプト |
| `NextPrompt` | 次のプロンプト |
| `QuickSelect` | QuickSelect モード |
| `ToggleViMode` | Vi モード |
| `ToggleSecureInput` | Secure Keyboard Entry のトグル |
| `NextTheme` | 次のテーマに切り替え |
| `PreviousTheme` | 前のテーマに切り替え |
| `ToggleDecorations` | ウィンドウ装飾のトグル |
| `ToggleAlwaysOnTop` | 常に最前面のトグル |
| `ToggleCommandPalette` | コマンドパレットのトグル |
| `Quit` | アプリケーション終了 |
| `About` | バージョン情報 |
| `Preferences` | 設定ファイルを開く |

### `unconsumed` オプション

`unconsumed = true` を指定すると、アクションを実行しつつ元のキーイベントをターミナルにも転送します。

デフォルトでは、キーバインドにマッチしたキーは SDIT が消費してターミナルには届きません。
`unconsumed = true` を使うと両方に届けることができます。

```toml
# Cmd+K でスクロールバッファをクリアしつつ、アプリへもキーを渡す
[[keybinds]]
key = "k"
mods = "super"
action = "ScrollToBottom"
unconsumed = true
```
