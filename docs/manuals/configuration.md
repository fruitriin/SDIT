# 設定リファレンス

設定ファイル: `~/.config/sdit/config.toml`

設定ファイルの変更はホットリロードで即座に反映されます。
値の範囲外の指定はデフォルト値にフォールバックします。

---

## `[font]` — フォント

| キー | 型 | デフォルト | 範囲 | 説明 |
|---|---|---|---|---|
| `family` | 文字列 | `"Menlo"` (macOS) | — | フォントファミリ名 |
| `size` | 小数 | `14.0` | 1.0–200.0 | フォントサイズ（ピクセル） |
| `line_height` | 小数 | `1.2` | 0.5–5.0 | 行の高さの倍率 |
| `fallback_families` | 文字列配列 | `[]` | — | フォールバックフォント（CJK 等） |
| `thicken` | 真偽値 | `false` | — | テキストを太くする |

### `[font.adjust]` — セル微調整

| キー | 型 | デフォルト | 範囲 | 説明 |
|---|---|---|---|---|
| `cell_width` | 小数 | `0.0` | -10.0–10.0 | セル幅調整（ピクセル） |
| `cell_height` | 小数 | `0.0` | -10.0–10.0 | セル高さ調整（ピクセル） |
| `baseline` | 小数 | `0.0` | -20.0–20.0 | ベースライン調整（ピクセル） |

### `[[font.codepoint_map]]` — コードポイント別フォント

特定の文字範囲に別のフォントを割り当てます。最大 64 エントリ。

```toml
[[font.codepoint_map]]
family = "Noto Color Emoji"
codepoint_start = 0x1F600
codepoint_end = 0x1F64F
```

### 例

```toml
[font]
family = "JetBrains Mono"
size = 16.0
line_height = 1.3
fallback_families = ["Hiragino Sans"]
thicken = true

[font.adjust]
cell_width = 1.0
baseline = -1.0
```

---

## `[colors]` — カラー

| キー | 型 | デフォルト | 範囲 | 説明 |
|---|---|---|---|---|
| `theme` | 文字列 | `"catppuccin-mocha"` | テーマ名 | カラーテーマ |
| `minimum_contrast` | 小数 | `1.0` | 1.0–21.0 | WCAG 最小コントラスト比（1.0=無効） |
| `bold_is_bright` | 真偽値 | `false` | — | 太字テキストを明色に変換 |
| `faint_opacity` | 小数 | `0.5` | 0.0–1.0 | SGR 2 (DIM) のアルファ倍率 |
| `selection_foreground` | 文字列 | なし | 16進数 | 選択テキスト前景色 |
| `selection_background` | 文字列 | なし | 16進数 | 選択テキスト背景色 |
| `search_foreground` | 文字列 | なし | 16進数 | 検索ハイライト前景色 |
| `search_background` | 文字列 | なし | 16進数 | 検索ハイライト背景色 |

利用可能なテーマは [テーマ](themes.md) を参照してください。

---

## `[window]` — ウィンドウ

| キー | 型 | デフォルト | 範囲 | 説明 |
|---|---|---|---|---|
| `opacity` | 小数 | `1.0` | 0.0–1.0 | 背景透明度 |
| `blur` | 真偽値 | `false` | — | 背景ブラー（macOS） |
| `padding_x` | 整数 | `0` | 0–200 | 左右パディング（ピクセル） |
| `padding_y` | 整数 | `0` | 0–200 | 上下パディング（ピクセル） |
| `padding_color` | 文字列 | `"background"` | background / extend | パディング背景色 |
| `columns` | 整数 | `80` | 10–500 | 初期ウィンドウ幅（列数） |
| `rows` | 整数 | `24` | 2–200 | 初期ウィンドウ高さ（行数） |
| `startup_mode` | 文字列 | `"Windowed"` | Windowed / Maximized / Fullscreen | 起動モード |
| `decorations` | 文字列 | `"full"` | full / none | ウィンドウ装飾 |
| `always_on_top` | 真偽値 | `false` | — | 常に最前面 |
| `confirm_close` | 文字列 | `"process_running"` | never / always / process_running | 閉じる時の確認 |
| `inherit_working_directory` | 真偽値 | `true` | — | 新セッション時にカレントディレクトリを継承 |
| `restore_session` | 真偽値 | `true` | — | 前回のセッションを復帰 |
| `working_directory` | 文字列 | なし | — | 初期作業ディレクトリ |
| `subtitle` | 文字列 | `"none"` | none / working-directory / session-name | ウィンドウサブタイトル |
| `position_x` | 整数 | なし | — | 初期ウィンドウ X 座標 |
| `position_y` | 整数 | なし | — | 初期ウィンドウ Y 座標 |

### 背景画像

| キー | 型 | デフォルト | 範囲 | 説明 |
|---|---|---|---|---|
| `background_image` | 文字列 | なし | パス | 背景画像（`~/` 展開可） |
| `background_image_opacity` | 小数 | `0.3` | 0.0–1.0 | 背景画像の透明度 |
| `background_image_fit` | 文字列 | `"cover"` | cover / contain / fill | 背景画像のフィット方法 |

---

## `[cursor]` — カーソル

| キー | 型 | デフォルト | 範囲 | 説明 |
|---|---|---|---|---|
| `style` | 文字列 | `"block"` | block / underline / bar | カーソル形状 |
| `blinking` | 真偽値 | `false` | — | カーソル点滅 |
| `color` | 文字列 | なし | 16進数 | カーソル色 |

---

## `[bell]` — ベル

| キー | 型 | デフォルト | 範囲 | 説明 |
|---|---|---|---|---|
| `visual` | 真偽値 | `true` | — | ビジュアルベル（画面フラッシュ） |
| `dock_bounce` | 真偽値 | `true` | — | Dock バウンス（macOS） |
| `duration_ms` | 整数 | `150` | — | フラッシュフェードアウト時間 |

---

## `[scrollback]` — スクロールバック

| キー | 型 | デフォルト | 範囲 | 説明 |
|---|---|---|---|---|
| `lines` | 整数 | `10000` | 0–1000000 | スクロールバック最大行数 |

---

## `[scrolling]` — スクロール

| キー | 型 | デフォルト | 範囲 | 説明 |
|---|---|---|---|---|
| `multiplier` | 整数 | `3` | 1–100 | ホイール 1 ノッチのスクロール行数 |
| `scroll_to_bottom_on_keystroke` | 真偽値 | `true` | — | キー入力時にボトムへスクロール |
| `scroll_to_bottom_on_output` | 真偽値 | `false` | — | 出力時にボトムへスクロール |

---

## `[scrollbar]` — スクロールバー

| キー | 型 | デフォルト | 範囲 | 説明 |
|---|---|---|---|---|
| `enabled` | 真偽値 | `true` | — | スクロールバー表示 |
| `width` | 整数 | `8` | 2–32 | スクロールバー幅（ピクセル） |

---

## `[selection]` — テキスト選択

| キー | 型 | デフォルト | 範囲 | 説明 |
|---|---|---|---|---|
| `save_to_clipboard` | 真偽値 | `false` | — | 選択時に自動クリップボードコピー |
| `trim_trailing_spaces` | 真偽値 | `true` | — | コピー時に末尾空白を削除 |
| `word_chars` | 文字列 | `""` | — | ダブルクリック時の単語に含める追加文字 |

---

## `[mouse]` — マウス

| キー | 型 | デフォルト | 範囲 | 説明 |
|---|---|---|---|---|
| `hide_when_typing` | 真偽値 | `false` | — | タイピング中にマウスカーソルを隠す |
| `right_click_action` | 文字列 | `"context_menu"` | context_menu / paste / none | 右クリック動作 |
| `click_repeat_interval` | 整数 | `300` | 50–2000 | ダブル/トリプルクリック判定時間（ms） |

---

## `[paste]` — ペースト

| キー | 型 | デフォルト | 範囲 | 説明 |
|---|---|---|---|---|
| `confirm_multiline` | 真偽値 | `true` | — | 複数行ペースト時の確認ダイアログ |

---

## `[option_as_alt]` — Option キー（macOS）

| キー | 型 | デフォルト | 範囲 | 説明 |
|---|---|---|---|---|
| 値 | 文字列 | `"none"` | none / both / only_left / only_right | Option を Alt として扱う |

```toml
[option_as_alt]
value = "both"
```

readline ショートカット（Alt+B で単語移動など）を使う場合に設定します。

---

## `[notification]` — 通知

| キー | 型 | デフォルト | 範囲 | 説明 |
|---|---|---|---|---|
| `enabled` | 真偽値 | `true` | — | デスクトップ通知を有効化 |
| `command_notify` | 文字列 | `"unfocused"` | never / unfocused / always | コマンド終了通知 |
| `command_notify_threshold` | 整数 | `10` | — | 通知対象のコマンド実行時間（秒） |

---

## `[shell_integration]` — シェルインテグレーション

| キー | 型 | デフォルト | 範囲 | 説明 |
|---|---|---|---|---|
| `enabled` | 真偽値 | `true` | — | OSC 133 プロンプトジャンプ |

---

## `[security]` — セキュリティ（macOS）

| キー | 型 | デフォルト | 範囲 | 説明 |
|---|---|---|---|---|
| `auto_secure_input` | 真偽値 | `false` | — | フォーカス時の Secure Keyboard Entry 自動有効化 |

---

## `[quick_terminal]` — Quick Terminal（macOS）

| キー | 型 | デフォルト | 範囲 | 説明 |
|---|---|---|---|---|
| `enabled` | 真偽値 | `false` | — | Quick Terminal を有効化 |
| `position` | 文字列 | `"top"` | top / bottom / left / right | スライドイン位置 |
| `size` | 小数 | `0.4` | 0.1–1.0 | 画面比率 |
| `hotkey` | 文字列 | `"ctrl+\`"` | — | グローバルホットキー |
| `animation_duration` | 小数 | `0.2` | 0.0–2.0 | アニメーション時間（秒） |

---

## `[terminal]` — ターミナル

| キー | 型 | デフォルト | 範囲 | 説明 |
|---|---|---|---|---|
| `grapheme_width_method` | 文字列 | `"unicode"` | unicode / legacy | 字幅計算方式 |
| `osc_color_report_format` | 文字列 | `"16-bit"` | 8-bit / 16-bit | OSC 10/11/12 応答形式 |
| `title_report` | 真偽値 | `false` | — | CSI 21t によるタイトル報告を許可 |
| `enquiry_response` | 文字列 | なし | 最大 256 文字 | ENQ (0x05) 応答文字列 |

---

## `[[keybinds]]` — キーバインド

[キーボードショートカット](keybinds.md) を参照してください。

```toml
[[keybinds]]
key = "j"
mods = "ctrl|shift"
action = "NextSession"
```
