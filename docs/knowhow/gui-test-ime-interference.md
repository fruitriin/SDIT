# GUI テスト時の日本語 IME 干渉問題

## 概要

macOS で `send-keys.sh`（AppleScript `keystroke`）を使って SDIT にキー入力を送信する際、
日本語 IME が有効だとコマンドが文字化けする。

## 問題の本質（実証済み）

AppleScript の `keystroke "Unicode文字列"` は OS の入力メソッドフレームワーク（IMK）を経由する。
**Google IME は英数モードに切り替えていても Unicode の CJK 文字を受け取ると変換処理を起動する。**

つまり `keystroke` で CJK 文字を送ることは **モード切り替えに関わらず根本的に不可能**。

```
cat を送信中に IME がひらがなモードだと…
  "c" → "カ"（カタカナ変換）

key code 102 で英数切り替え後でも…
  keystroke "こんにちわ世界" → IME 変換ポップアップが出る（"あああああ" 候補が表示）
```

**ASCII 文字（`echo` 等）は英数切り替え後に `keystroke` で送れる。**
**CJK 文字は `keystroke` では送れない。PTY 直接書き込みを使うこと。**

## 英数切り替えの方法と注意点

### key code 102（英数キー）

JIS キーボードの英数キー。**ただし IME の設定によってキーコードが異なる場合がある。**

```applescript
key code 102  -- JIS 英数キー
delay 0.2     -- 切り替わりを待つ（0.1s では不十分な場合がある）
```

**Google 日本語入力など IME ソフトによってはこのキーコードで切り替わらない場合がある。**
切り替わったかどうかは実際に試して確認すること。

### 入力ソースを直接切り替える（より確実）

```bash
# ABC / 英数入力ソースに直接切り替える
osascript -e 'tell application "System Events" to set the input source to "com.apple.keylayout.ABC"' 2>/dev/null

# または Google IME の英数入力ソース識別子を使う
osascript -e 'tell application "System Events"
    set inputSources to every input source whose selected is true
    set focused UI element of (first process whose name is "sdit") to missing value
end tell' 2>/dev/null
```

### 確実な確認方法

```bash
# 現在のアクティブな入力ソースを確認
defaults read ~/Library/Preferences/com.apple.HIToolbox.plist AppleSelectedInputSources 2>/dev/null \
  | grep -E '"Bundle ID"|"Input Mode"'
```

`Bundle ID = com.apple.keylayout.ABC` や `Input Mode = ""` になっていれば英数モード。

## 回避策（確実度の高い順）

### 1. 入力ソース直接切り替え（最も確実）

```bash
osascript -e 'tell application "System Events" to set the input source to "com.apple.keylayout.ABC"'
sleep 0.3
./tools/test-utils/send-keys.sh sdit "echo こんにちわ世界"
```

### 2. クリップボード経由（IME を完全バイパス）

```bash
printf "echo こんにちわ世界" | pbcopy
osascript -e 'tell application "System Events" to keystroke "v" using command down'
# ⚠️ ブラケテッドペーストが発生するため SDIT のペーストモード対応に依存
```

### 3. PTY 直接書き込み（最も安定・IME 無関係）

```bash
TTY=$(ps -p <PID> -o tty= | tr -d ' ')
printf "echo こんにちわ世界\r" > /dev/$TTY
# AppleScript・IME・権限を一切経由しない
```

### 4. key code 102 先打ち（IME 設定依存）

```bash
osascript -e 'tell application "System Events" to key code 102'
sleep 0.2
./tools/test-utils/send-keys.sh sdit "echo こんにちわ世界"
# Google IME など IME によっては切り替わらない場合がある
```

## 推奨方針（実証済み）

| 入力内容 | 推奨手段 | 理由 |
|---|---|---|
| ASCII のみ | key code 102 先打ち + `send-keys.sh` | 動作安定 |
| CJK 文字を含む | **PTY 直接書き込み一択** | 下記参照 |

### CJK 入力は PTY 直接書き込み一択

`send-keys.sh`（AppleScript keystroke）+ Google IME の組み合わせでは、
どのようなアプローチを試みても CJK コマンドの実行が安定しない：

- `key code 102` で英数切り替え → スペースが IME に消費される
- `echo` → ` ` → `こんにちわ世界` を分割送信 → Enter が何回送っても吸われ続ける
- `IMKCFRunLoopWakeUpReliable` エラーが発生し fish + IME が不安定な状態になる

**PTY 直接書き込みを使うこと：**

```bash
# PTY デバイスを特定
TTY=$(ps -p <PID> -o tty= | tr -d ' ')

# コマンドを直接書き込む（\r = Enter）
printf "echo こんにちわ世界\r" > /dev/$TTY
```

PTY 直接書き込みは AppleScript・IME・権限を一切経由しないため完全に安定する。

## スペースが消える問題（実測）

`key code 102` で英数モードに切り替えても、切り替えタイミングのずれで
コマンド中のスペースが IME の「変換トリガー」として吸われることがある。

```
"echo こんにちわ世界" を送信 → "echoこんにちわ世界" になる
（スペースが変換トリガーとして消えた）
```

**対策**: スペースを含むコマンドは key code 102 切り替え後に十分な delay (0.3s 以上) を置くか、
PTY 直接書き込みを使うこと。

## 実測確認済みの動作（2026-03-14）

- `key code 102` + 0.3s delay → `defaults read` で入力ソース切り替えを確認済み
- CJK 文字自体は SDIT に届く（グリフも正常描画される）
- ただしスペース文字が IME に吸われる問題が残る

**SDIT の CJK レンダリング自体は問題なし**（SSIM 0.9927 確認済み）。
IME との戦いはテスト基盤の問題であり、SDIT 本体の問題ではない。

## 注意点

- SDIT は `set_ime_allowed(true)` を呼んでいるため、ウィンドウ側で IME を無効化できない
- `key code 102` は JIS キーボードの英数キー。**Google IME など IME ソフトの設定次第で動作しない場合がある**
- クリップボード経由の場合、ブラケテッドペーストが発生しうる
- PTY 直接書き込みは tty 番号が起動ごとに変わるため `ps` で都度確認が必要
