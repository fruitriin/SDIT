# GUI テスト時の日本語 IME 干渉問題

## 概要

macOS で `send-keys.sh`（AppleScript `keystroke`）を使って SDIT にキー入力を送信する際、日本語 IME が有効だとコマンドが文字化けする。

## 原因

AppleScript の `keystroke` は OS の入力メソッドを経由するため、日本語入力モードだと ASCII 文字が変換されてしまう。

## 回避策（基本方針）

**毎回 `send-keys.sh` を呼ぶ前に英数キー（key code 102）を送信して IME モードを確定する。**

現在の IME モードに依存せず、常に ASCII 入力モードに切り替えることで文字化けを防ぐ。
「現在すでに英数モードのはず」と思っていても、テスト間の状態が持ち越されることがあるため、
**必ず毎回リセットする習慣をつけること**。

```bash
# 推奨パターン: ASCII コマンドを送る前に必ず英数キーを先打ち
./tools/test-utils/send-keys.sh sdit $'\x1b[102]'  # 英数キー（key code 102）

# または send-keys.sh が英数キーを内部で送れるよう実装する場合:
./tools/test-utils/send-keys.sh --eisu sdit "echo hello"
```

`send-keys.sh` の内部 AppleScript では:
```applescript
key code 102  -- 英数キーで ASCII モードに切り替え
delay 0.1
keystroke "echo hello"
```

**変換キー（key code 49 on JIS）も同様の効果**があり、IME を確定（コミット）してから次の入力に進むために使える。ただし通常は英数キーで十分。

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
