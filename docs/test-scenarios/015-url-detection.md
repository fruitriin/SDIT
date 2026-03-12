# 015: URL 検出・Cmd+クリック

## 目的

ターミナル出力内の URL を検出し、Cmd+クリックでブラウザを開く機能、および Cmd ホバー時の視覚的フィードバック（青色アンダーライン + カーソル変更）が正常に動作することを確認する。

## 前提条件

- `cargo build --package sdit`
- `tools/test-utils/build.sh`
- Screen Recording 権限 + OS 再起動
- Accessibility 権限

## 手順

### 015-1: 正規表現による URL 検出 — Cmd ホバーで視覚的フィードバック

1. SDIT をバックグラウンドで起動する
2. window-info でウィンドウの存在を確認する（最大 15 秒ポーリング）
3. IME 干渉を防ぐため `key code 102`（英数キー）を送信して英語入力モードにする（delay 0.3）
4. send-keys で `echo https://example.com` を入力して Return キーを送信する
5. 1 秒待機してベースライン画像をキャプチャする（`tmp/015-base.png`）
6. osascript で Cmd キーを押下した状態で URL 上にマウスカーソルを移動する
7. 1 秒待機してスクリーンショットを撮る（`tmp/015-hover.png`）
8. ベースラインとホバー画像のファイルサイズが異なることを確認（URL がハイライトされている）

### 015-2: Cmd+クリックで URL を開く

1. （015-1 の継続）SDIT ウィンドウにフォーカスを当てる
2. send-keys で `echo https://httpbin.org/get` を入力して Return キーを送信する
3. 1 秒待機する
4. osascript で Cmd キーを押下しながら URL 上をクリックする
5. 2 秒待機する
6. window-info で SDIT ウィンドウがまだ存在することを確認する（クラッシュしていない）
7. （ブラウザが開いたかは目視確認 — 自動検証は困難）

### 015-3: OSC 8 ハイパーリンク検出

1. SDIT ウィンドウにフォーカスを当てる
2. OSC 8 エスケープシーケンスで URL 付きテキストを出力する:
   `printf '\e]8;;https://osc8.example.com\e\\OSC8_LINK\e]8;;\e\\'`
3. 1 秒待機する
4. capture-window でスクリーンショットを撮る（`tmp/015-osc8.png`）
5. osascript で Cmd キーを押下した状態で "OSC8_LINK" テキスト上にマウスカーソルを移動する
6. 1 秒待機してスクリーンショットを撮る（`tmp/015-osc8-hover.png`）

### 015-4: URL がない行でホバーしてもハイライトされない

1. send-keys で `echo no_url_here` を入力して Return キーを送信する
2. 1 秒待機する
3. osascript で Cmd キーを押下した状態で "no_url_here" テキスト上にマウスカーソルを移動する
4. 1 秒待機してスクリーンショットを撮る（`tmp/015-no-url.png`）
5. ベースライン（`tmp/015-base.png`）とのファイルサイズ比較で大きな変化がないことを確認

### 015-5: 長い URL / 特殊文字を含む URL

1. send-keys で `echo https://example.com/path?q=hello&r=world#section` を入力して Return キーを送信する
2. 1 秒待機する
3. osascript で Cmd キーを押下した状態で URL 上にマウスカーソルを移動する
4. 1 秒待機してスクリーンショットを撮る（`tmp/015-long-url.png`）
5. SDIT がクラッシュしていないことを window-info で確認する

### 015-6: http:// (非 HTTPS) URL の検出

1. send-keys で `echo http://insecure.example.com` を入力して Return キーを送信する
2. 1 秒待機する
3. osascript で Cmd キーを押下した状態で URL 上にマウスカーソルを移動する
4. 1 秒待機してスクリーンショットを撮る（`tmp/015-http.png`）

## 期待結果

### 015-1
- `tmp/015-base.png` のファイルサイズが 10 KiB 以上（空白でない）
- `tmp/015-hover.png` のファイルサイズが 10 KiB 以上
- ホバー画像で URL 部分に青色アンダーラインが表示されている（目視確認）
- SDIT プロセスがクラッシュしていない（window-info が exit 0）

### 015-2
- SDIT プロセスがクラッシュしていない
- デフォルトブラウザで URL が開かれる（目視確認）

### 015-3
- `tmp/015-osc8.png` に "OSC8_LINK" テキストが表示されている
- Cmd ホバー時に OSC 8 ハイパーリンクのアンダーラインが表示される（目視確認）

### 015-4
- URL がない行ではハイライト変化がない

### 015-5
- 長い URL + クエリパラメータ + フラグメントを含む URL でクラッシュしない
- URL 全体がハイライトされる（目視確認）

### 015-6
- http:// スキームの URL も検出される
- ホバー時にハイライトされる（目視確認）

## クリーンアップ

- SDIT プロセスを終了する
- `tmp/015-*.png` を削除する
- テスト中に開かれたブラウザタブを閉じる

## 実行スクリプト例

```bash
#!/bin/bash
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

SDIT_PID=""
cleanup() {
    [ -n "$SDIT_PID" ] && kill "$SDIT_PID" 2>/dev/null || true
    rm -f tmp/015-*.png
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

# --- 015-1: URL 表示 + ベースライン ---
./tools/test-utils/send-keys.sh sdit "echo https://example.com"
osascript -e 'tell application "System Events" to set frontmost of (first process whose name is "sdit") to true'
osascript -e 'tell application "System Events" to keystroke return'
sleep 1
./tools/test-utils/capture-window sdit tmp/015-base.png

# Cmd ホバー: window-info でウィンドウ座標を取得し、URL 位置にマウスを移動
INFO=$(./tools/test-utils/window-info sdit)
WIN_X=$(echo "$INFO" | python3 -c "import sys,json; d=json.load(sys.stdin); print(int(d['position']['x']))")
WIN_Y=$(echo "$INFO" | python3 -c "import sys,json; d=json.load(sys.stdin); print(int(d['position']['y']))")
# URL は "echo https://example.com" の出力行（2行目付近）、横位置は中央付近
# フォントサイズ約 14pt = セル幅約 8px, セル高さ約 18px として概算
URL_PIXEL_X=$((WIN_X + 150 + 80))  # サイドバーオフセット + URL 中央付近
URL_PIXEL_Y=$((WIN_Y + 40 + 18))   # タイトルバー + 出力行の Y 位置
osascript <<APPLESCRIPT
tell application "System Events"
    -- Cmd キーを押しながらマウス移動（フラグ: command down）
    -- 注: AppleScript でマウス移動 + modifier は直接サポートされないため、
    -- cliclick 等の外部ツールが必要。簡易的に Cmd 押下のみ行う
    key down command
end tell
APPLESCRIPT
# cliclick がある場合: cliclick m:$URL_PIXEL_X,$URL_PIXEL_Y
sleep 1
./tools/test-utils/capture-window sdit tmp/015-hover.png
osascript -e 'tell application "System Events" to key up command'

# --- 015-2: Cmd+Click ---
./tools/test-utils/send-keys.sh sdit "echo https://httpbin.org/get"
osascript -e 'tell application "System Events" to keystroke return'
sleep 1
# Cmd+Click は AppleScript で直接実行が難しいため、目視テスト推奨
# cliclick がある場合: cliclick kd:cmd c:$URL_PIXEL_X,$URL_PIXEL_Y ku:cmd

# --- 015-3: OSC 8 ハイパーリンク ---
./tools/test-utils/send-keys.sh sdit "printf '\\e]8;;https://osc8.example.com\\e\\\\OSC8_LINK\\e]8;;\\e\\\\'"
osascript -e 'tell application "System Events" to keystroke return'
sleep 1
./tools/test-utils/capture-window sdit tmp/015-osc8.png

# --- 015-4: URL なし行 ---
./tools/test-utils/send-keys.sh sdit "echo no_url_here"
osascript -e 'tell application "System Events" to keystroke return'
sleep 1
./tools/test-utils/capture-window sdit tmp/015-no-url.png

# --- 015-5: 長い URL ---
./tools/test-utils/send-keys.sh sdit "echo https://example.com/path?q=hello&r=world#section"
osascript -e 'tell application "System Events" to keystroke return'
sleep 1
./tools/test-utils/capture-window sdit tmp/015-long-url.png

# --- 015-6: http URL ---
./tools/test-utils/send-keys.sh sdit "echo http://insecure.example.com"
osascript -e 'tell application "System Events" to keystroke return'
sleep 1
./tools/test-utils/capture-window sdit tmp/015-http.png

# クラッシュチェック
./tools/test-utils/window-info sdit >/dev/null

# 結果確認
FAIL=0
for f in tmp/015-base.png tmp/015-osc8.png tmp/015-no-url.png tmp/015-long-url.png tmp/015-http.png; do
    SIZE=$(wc -c < "$f")
    if [ "$SIZE" -lt 10240 ]; then
        echo "FAIL: $f is too small ($SIZE bytes)"
        FAIL=1
    else
        echo "OK: $f ($SIZE bytes)"
    fi
done

if [ $FAIL -eq 0 ]; then
    echo "All 015 automated checks passed."
    echo "NOTE: Cmd+hover highlight and Cmd+click browser open require manual verification."
fi
```

## 制限事項

- **Cmd+マウスホバー**: AppleScript では modifier + マウス移動の組み合わせが直接サポートされない。`cliclick` 等の外部ツール、または手動テストが必要
- **Cmd+クリック**: 同様に AppleScript での Cmd+クリック送信は困難。手動テスト推奨
- **ブラウザ起動の自動検証**: `open` コマンドの実行自体は検証困難。プロセスがクラッシュしないことのみ自動検証

## 関連

- Phase 8.2: `docs/plans/phase8.2-url-detection.md`
- `crates/sdit-core/src/terminal/url_detector.rs` — UrlDetector
- `crates/sdit-core/src/grid/cell.rs` — Cell.hyperlink フィールド
- `crates/sdit/src/event_loop.rs` — update_url_hover(), Cmd+Click ハンドラ
- `crates/sdit/src/app.rs` — UrlHoverState
