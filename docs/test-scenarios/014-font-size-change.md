# 014: フォントサイズ動的変更（Cmd+=/Cmd+-/Cmd+0）

## 目的

Cmd+= でフォントサイズ拡大、Cmd+- でフォントサイズ縮小、Cmd+0 でデフォルトサイズに復帰する操作が
クラッシュなく正常に動作し、描画が更新されることを確認する。

## 前提条件

- `cargo build --package sdit`
- `tools/test-utils/build.sh`
- Screen Recording 権限 + OS 再起動
- Accessibility 権限

## 手順

### 014-1: Cmd+= でフォントサイズ拡大

1. SDIT をバックグラウンドで起動する
2. window-info でウィンドウの存在を確認する（最大 15 秒ポーリング）
3. IME 干渉を防ぐため `key code 102`（英数キー）を送信して英語入力モードにする（delay 0.3）
4. send-keys で `echo FONT_SIZE_BASE` を入力して Return キーを送信する
5. 1 秒待機してベースライン画像をキャプチャする（`tmp/014-base.png`）
6. osascript で `keystroke "=" using command down` を送信（Cmd+=）
7. 1 秒待機する（アトラスクリア → 再ラスタライズ → 描画更新）
8. capture-window でスクリーンショットを撮る（`tmp/014-zoom-in.png`）
9. さらに Cmd+= を 3 回送信する（連続操作）
10. 1 秒待機してスクリーンショットを撮る（`tmp/014-zoom-in-max.png`）

### 014-2: Cmd+- でフォントサイズ縮小

1. （014-1 の継続、または再起動後）SDIT ウィンドウにフォーカスを当てる
2. IME 干渉を防ぐため `key code 102` を送信する
3. osascript で `keystroke "-" using command down` を送信（Cmd+-）
4. 1 秒待機してスクリーンショットを撮る（`tmp/014-zoom-out.png`）
5. さらに Cmd+- を 3 回送信する（連続操作）
6. 1 秒待機してスクリーンショットを撮る（`tmp/014-zoom-out-min.png`）

### 014-3: Cmd+0 でデフォルトサイズ復帰

1. （014-2 の継続）Cmd+0 を送信（`keystroke "0" using command down`）
2. 1 秒待機してスクリーンショットを撮る（`tmp/014-reset.png`）
3. send-keys で `echo FONT_RESET` を入力して Return キーを送信する
4. 1 秒待機してスクリーンショットを撮る（`tmp/014-reset-text.png`）

### 014-4: 連続操作でクラッシュしないこと

1. Cmd+= を 10 回、Cmd+- を 10 回、Cmd+0 を 1 回連続で素早く送信する
   （各送信の間に delay 0.1）
2. 2 秒待機する
3. window-info でウィンドウがまだ存在することを確認する（exit 0）
4. send-keys で `echo STILL_ALIVE` を入力して Return キーを送信する
5. 1 秒待機してスクリーンショットを撮る（`tmp/014-stress.png`）

## 自動検証（verify-text）

```bash
# ベースラインテキスト確認
./tools/test-utils/verify-text tmp/014-base.png "FONT_SIZE_BASE"

# ストレステスト後の生存確認
./tools/test-utils/verify-text tmp/014-stress.png "STILL_ALIVE"
```

- フォントサイズ変更の前後で OCR が通れば、テキストが正常に描画されていることが確定する
- `--cells` / `--reference` は不要（ASCII テキストの存在確認が目的）

## 期待結果

### 014-1
- `tmp/014-zoom-in.png` のファイルサイズが 10 KiB 以上（空白でない）
- ベースライン画像（`tmp/014-base.png`）と拡大後の画像でファイルサイズが異なる
  （フォントが大きくなり描画内容が変化している）
- **verify-text が exit 0**（OCR で "FONT_SIZE_BASE" が認識される）
- SDIT プロセスがクラッシュしていない（window-info が exit 0）

### 014-2
- `tmp/014-zoom-out.png` のファイルサイズが 10 KiB 以上
- SDIT プロセスがクラッシュしていない

### 014-3
- `tmp/014-reset.png` のファイルサイズが 10 KiB 以上
- `tmp/014-reset-text.png` のファイルサイズが 10 KiB 以上
- SDIT プロセスがクラッシュしていない

### 014-4
- window-info が exit 0（ウィンドウが生存している）
- `tmp/014-stress.png` のファイルサイズが 10 KiB 以上
- **verify-text が exit 0**（OCR で "STILL_ALIVE" が認識される）

## クリーンアップ

- SDIT プロセスを終了する
- `tmp/014-*.png` を削除する

## 実行スクリプト例

```bash
#!/bin/bash
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

SDIT_PID=""
cleanup() {
    [ -n "$SDIT_PID" ] && kill "$SDIT_PID" 2>/dev/null || true
    rm -f tmp/014-*.png
}
trap cleanup EXIT

# 起動
pkill -f "target/debug/sdit" 2>/dev/null || true
sleep 0.5
./target/debug/sdit &
SDIT_PID=$!

# ウィンドウ待機（最大 15 秒）
for i in $(seq 1 30); do
    if ./tools/test-utils/window-info sdit >/dev/null 2>&1; then break; fi
    sleep 0.5
done
./tools/test-utils/window-info sdit >/dev/null

# IME 干渉回避
osascript -e 'tell application "System Events" to key code 102'
sleep 0.3

# ベースライン
./tools/test-utils/send-keys.sh sdit "echo FONT_SIZE_BASE"
osascript -e 'tell application "System Events" to set frontmost of (first process whose name is "sdit") to true'
osascript -e 'tell application "System Events" to keystroke return'
sleep 1
./tools/test-utils/capture-window sdit tmp/014-base.png

# Cmd+= 拡大
osascript -e 'tell application "System Events" to keystroke "=" using command down'
sleep 1
./tools/test-utils/capture-window sdit tmp/014-zoom-in.png

# 連続拡大
for i in 1 2 3; do
    osascript -e 'tell application "System Events" to keystroke "=" using command down'
    sleep 0.1
done
sleep 1
./tools/test-utils/capture-window sdit tmp/014-zoom-in-max.png

# Cmd+- 縮小
osascript -e 'tell application "System Events" to keystroke "-" using command down'
sleep 1
./tools/test-utils/capture-window sdit tmp/014-zoom-out.png

# Cmd+0 リセット
osascript -e 'tell application "System Events" to keystroke "0" using command down'
sleep 1
./tools/test-utils/capture-window sdit tmp/014-reset.png

# ストレステスト
for i in $(seq 1 10); do
    osascript -e 'tell application "System Events" to keystroke "=" using command down'
    sleep 0.1
done
for i in $(seq 1 10); do
    osascript -e 'tell application "System Events" to keystroke "-" using command down'
    sleep 0.1
done
osascript -e 'tell application "System Events" to keystroke "0" using command down'
sleep 2

./tools/test-utils/window-info sdit >/dev/null  # クラッシュしていないか確認
./tools/test-utils/send-keys.sh sdit "echo STILL_ALIVE"
osascript -e 'tell application "System Events" to keystroke return'
sleep 1
./tools/test-utils/capture-window sdit tmp/014-stress.png

# 結果確認
FAIL=0
for f in tmp/014-base.png tmp/014-zoom-in.png tmp/014-zoom-out.png tmp/014-reset.png tmp/014-stress.png; do
    SIZE=$(wc -c < "$f")
    if [ "$SIZE" -lt 10240 ]; then
        echo "FAIL: $f is too small ($SIZE bytes)"
        FAIL=1
    else
        echo "OK: $f ($SIZE bytes)"
    fi
done

if [ $FAIL -eq 0 ]; then
    echo "All 014 checks passed."
fi
```

## 関連

- Phase 8.1: `docs/plans/phase8.1-font-size.md`
- `crates/sdit-core/src/render/atlas.rs` — Atlas::clear()
- `crates/sdit-core/src/font/` — FontContext::set_font_size()
- `crates/sdit/src/app.rs` — change_font_size()
- `crates/sdit/src/event_loop.rs` — ショートカットハンドラ
