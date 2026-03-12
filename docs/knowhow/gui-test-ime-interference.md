# GUI テスト時の日本語 IME 干渉問題

## 概要

macOS で `send-keys.sh`（AppleScript `keystroke`）を使って SDIT にキー入力を送信する際、日本語 IME が有効だとコマンドが文字化けする。

## 原因

AppleScript の `keystroke` は OS の入力メソッドを経由するため、日本語入力モードだと ASCII 文字が変換されてしまう。

## 回避策（基本方針）

`keystroke` の前に英数キー（key code 102）を送信して英語モードに切り替える。

```applescript
key code 102  -- 英数キーで英語入力に切り替え
delay 0.3
keystroke "tput cols"
```

## 代替手段: クリップボード経由

`pbcopy` + `Cmd+V` で送信すれば IME を完全にバイパスできる。

```bash
echo -n "command text" | pbcopy
osascript -e 'keystroke "v" using command down'
```

## 注意点

- SDIT は `set_ime_allowed(true)` を呼んでいるため、ウィンドウ側で IME を無効化することはできない
- クリップボード経由の場合、ブラケテッドペースト（`\e[200~...\e[201~`）が発生する可能性がある。テスト対象のアプリケーションがペーストモードに対応しているか確認すること
- `key code 102` は JIS キーボードの英数キー。US キーボードの場合は別のアプローチが必要
