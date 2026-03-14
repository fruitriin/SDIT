# 027: vi モード（コピーモード）

## 目的

Cmd+Shift+V で vi モード（コピーモード）を起動し、hjkl 基本移動・v 選択・y ヤンク・/ 検索連携・Escape 終了が動作することを確認する。

## 前提条件

- `cargo build --package sdit`
- `tools/test-utils/build.sh`
- Screen Recording 権限 + Accessibility 権限
- ディスプレイがアクティブ状態（Display Asleep: No）

## 手順

### 027-1: vi モード起動と終了（Cmd+Shift+V / Escape）

1. SDIT をバックグラウンドで起動する
2. window-info でウィンドウの存在を確認する（最大 15 秒ポーリング）
3. IME 干渉を防ぐため `key code 102`（英数キー）を送信して英語入力モードにする（delay 0.3）
4. send-keys で `echo "hello world"` を入力して Return キーを送信する
5. 1 秒待機してベースライン画像をキャプチャする（`tmp/027-base.png`）
6. Cmd+Shift+V を送信して vi モードを起動する
7. 0.5 秒待機してスクリーンショットをキャプチャする（`tmp/027-vi-active.png`）
8. ベースラインとの差分確認: vi カーソル（ブロック形状）が表示されている（目視）
9. Escape キーを送信して vi モードを終了する
10. 0.5 秒待機してスクリーンショットをキャプチャする（`tmp/027-vi-exit.png`）
11. vi カーソルが消えていることを確認する（目視）
12. SDIT プロセスがクラッシュしていないことを window-info で確認する

### 027-2: hjkl 基本移動

1. （027-1 の SDIT を継続して使用）
2. send-keys で複数行のコンテンツを出力する:
   ```
   echo "line1 aaa"
   echo "line2 bbb"
   echo "line3 ccc"
   ```
3. 1 秒待機する
4. Cmd+Shift+V を送信して vi モードを起動する
5. 0.5 秒待機してスクリーンショットをキャプチャする（`tmp/027-hjkl-start.png`）
6. `k` キーを 2 回送信して上に移動する
7. 0.5 秒待機してスクリーンショットをキャプチャする（`tmp/027-hjkl-up.png`）
8. カーソルが上に移動している（目視確認）
9. `j` キーを 1 回送信して下に移動する
10. `l` キーを 3 回送信して右に移動する
11. `h` キーを 1 回送信して左に移動する
12. 0.5 秒待機してスクリーンショットをキャプチャする（`tmp/027-hjkl-moved.png`）
13. Escape で vi モードを終了する

### 027-3: v で選択開始、y でヤンク

1. （027-2 の SDIT を継続して使用）
2. send-keys で `echo "copy this text"` を入力して Return キーを送信する
3. 1 秒待機する
4. Cmd+Shift+V を送信して vi モードを起動する
5. `0` キーを送信して行頭に移動する
6. `v` キーを送信して文字選択モードを開始する
7. 0.3 秒待機する
8. `l` キーを 4 回送信して 5 文字分選択する
9. 0.5 秒待機してスクリーンショットをキャプチャする（`tmp/027-selection.png`）
10. 選択範囲がハイライト表示されている（目視確認）
11. `y` キーを送信してヤンクする
12. 0.5 秒待機する
13. vi モードが終了していることを確認する（スクリーンショット `tmp/027-after-yank.png`）
14. send-keys で `pbpaste` を入力して Return キーを送信する
15. 1 秒待機してスクリーンショットをキャプチャする（`tmp/027-pbpaste.png`）
16. クリップボードに選択テキストが入っていることを目視確認する

### 027-4: V で行選択

1. （027-3 の SDIT を継続して使用）
2. send-keys で `echo "line selection test"` を入力して Return キーを送信する
3. 1 秒待機する
4. Cmd+Shift+V を送信して vi モードを起動する
5. `V` キー（大文字）を送信して行選択モードを開始する
6. 0.5 秒待機してスクリーンショットをキャプチャする（`tmp/027-line-sel.png`）
7. 行全体がハイライト表示されている（目視確認）
8. `y` キーを送信してヤンクする
9. 0.5 秒待機する
10. vi モードが終了していることを確認する
11. SDIT プロセスがクラッシュしていないことを window-info で確認する

### 027-5: / で検索連携

1. （027-4 の SDIT を継続して使用）
2. send-keys で `echo "search target word"` を入力して Return キーを送信する
3. 1 秒待機する
4. Cmd+Shift+V を送信して vi モードを起動する
5. `/` キーを送信して検索モードを起動する
6. 0.5 秒待機してスクリーンショットをキャプチャする（`tmp/027-search-active.png`）
7. 検索バー（入力フィールド）が表示されている（目視確認）
8. send-keys で `target` を入力して Return キーを送信する
9. 0.5 秒待機してスクリーンショットをキャプチャする（`tmp/027-search-result.png`）
10. "target" が検索結果としてハイライト表示されている（目視確認）
11. Escape で検索・vi モードを終了する
12. SDIT プロセスがクラッシュしていないことを window-info で確認する

### 027-6: vi カーソル描画確認（通常カーソルとの区別）

1. SDIT を再起動する
2. window-info でウィンドウの存在を確認する（最大 15 秒ポーリング）
3. IME 干渉を防ぐため `key code 102` を送信する（delay 0.3）
4. 起動直後のスクリーンショットをキャプチャする（`tmp/027-normal-cursor.png`）
5. Cmd+Shift+V を送信して vi モードを起動する
6. 0.5 秒待機してスクリーンショットをキャプチャする（`tmp/027-vi-cursor.png`）
7. 2つのスクリーンショットのカーソル形状が異なることを目視確認する
   - 通常: ライン/ビーム型カーソル（設定による）
   - vi モード: ブロック型カーソルが固定表示
8. Escape で vi モードを終了する
9. SDIT プロセスがクラッシュしていないことを window-info で確認する

## 期待結果

### 027-1
- `tmp/027-vi-active.png` で vi カーソル（ブロック形状）が表示されている
- `tmp/027-vi-exit.png` で vi カーソルが消えている
- SDIT プロセスがクラッシュしていない

### 027-2
- `tmp/027-hjkl-up.png` でカーソルが上行に移動している
- `tmp/027-hjkl-moved.png` でカーソルが hjkl 操作後の位置に表示されている
- SDIT プロセスがクラッシュしていない

### 027-3
- `tmp/027-selection.png` で選択範囲がハイライトされている
- `tmp/027-after-yank.png` で vi モードが終了している（vi カーソルが消えている）
- `pbpaste` の出力にヤンクしたテキストが含まれている

### 027-4
- `tmp/027-line-sel.png` で行全体がハイライトされている
- ヤンク後に vi モードが終了している

### 027-5
- `tmp/027-search-active.png` で検索バーが表示されている
- `tmp/027-search-result.png` で検索結果がハイライトされている

### 027-6
- `tmp/027-normal-cursor.png` と `tmp/027-vi-cursor.png` でカーソル形状が異なる
- vi モードではブロックカーソルが表示されている

## クリーンアップ

- SDIT プロセスを終了する
- `tmp/027-*.png` を削除する

## 実行スクリプト例

```bash
#!/bin/bash
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

# ディスプレイスリープ確認
if system_profiler SPDisplaysDataType 2>/dev/null | grep -q "Display Asleep: Yes"; then
    echo "SKIP: Display is asleep — GUI tests cannot run. Record as UNIT_ONLY."
    exit 0
fi

SDIT_BIN="./target/debug/sdit"
SDIT_PID=""
cleanup() {
    [ -n "$SDIT_PID" ] && kill "$SDIT_PID" 2>/dev/null || true
    rm -f tmp/027-*.png
}
trap cleanup EXIT

mkdir -p tmp

# 起動
pkill -x sdit 2>/dev/null || true
sleep 0.3
RUST_LOG=info "$SDIT_BIN" &
SDIT_PID=$!

# ウィンドウ待機（最大 15 秒）
for i in $(seq 1 30); do
    if ./tools/test-utils/window-info --pid "$SDIT_PID" >/dev/null 2>&1; then
        WIN_INFO=$(./tools/test-utils/window-info --pid "$SDIT_PID")
        W=$(echo "$WIN_INFO" | python3 -c "import sys,json; d=json.load(sys.stdin); print(int(d['size']['width']))")
        [ "$W" -gt 0 ] && break
    fi
    sleep 0.5
done

WIN_INFO=$(./tools/test-utils/window-info --pid "$SDIT_PID")
echo "Window info: $WIN_INFO"

# IME 干渉回避
osascript -e 'tell application "System Events" to key code 102'
sleep 0.3

# フォーカス
osascript -e "tell application \"System Events\" to set frontmost of (first process whose unix id is $SDIT_PID) to true"
sleep 0.3

# --- 027-1: vi モード起動と終了 ---
./tools/test-utils/send-keys.sh --pid "$SDIT_PID" "echo 'hello world'"
osascript -e 'tell application "System Events" to keystroke return'
sleep 1
./tools/test-utils/capture-window --pid "$SDIT_PID" tmp/027-base.png

# Cmd+Shift+V で vi モード起動
osascript -e 'tell application "System Events" to keystroke "v" using {command down, shift down}'
sleep 0.5
./tools/test-utils/capture-window --pid "$SDIT_PID" tmp/027-vi-active.png

# Escape で vi モード終了
osascript -e 'tell application "System Events" to key code 53'
sleep 0.5
./tools/test-utils/capture-window --pid "$SDIT_PID" tmp/027-vi-exit.png

# --- 027-2: hjkl 移動 ---
./tools/test-utils/send-keys.sh --pid "$SDIT_PID" "echo 'line1 aaa'"
osascript -e 'tell application "System Events" to keystroke return'
./tools/test-utils/send-keys.sh --pid "$SDIT_PID" "echo 'line2 bbb'"
osascript -e 'tell application "System Events" to keystroke return'
./tools/test-utils/send-keys.sh --pid "$SDIT_PID" "echo 'line3 ccc'"
osascript -e 'tell application "System Events" to keystroke return'
sleep 1

osascript -e 'tell application "System Events" to keystroke "v" using {command down, shift down}'
sleep 0.5
./tools/test-utils/capture-window --pid "$SDIT_PID" tmp/027-hjkl-start.png

# k キーで上移動
osascript -e 'tell application "System Events" to keystroke "k"'
sleep 0.1
osascript -e 'tell application "System Events" to keystroke "k"'
sleep 0.5
./tools/test-utils/capture-window --pid "$SDIT_PID" tmp/027-hjkl-up.png

# j, l, h 移動
osascript -e 'tell application "System Events" to keystroke "j"'
sleep 0.1
osascript -e 'tell application "System Events" to keystroke "l"'
sleep 0.1
osascript -e 'tell application "System Events" to keystroke "l"'
sleep 0.1
osascript -e 'tell application "System Events" to keystroke "l"'
sleep 0.1
osascript -e 'tell application "System Events" to keystroke "h"'
sleep 0.5
./tools/test-utils/capture-window --pid "$SDIT_PID" tmp/027-hjkl-moved.png

osascript -e 'tell application "System Events" to key code 53'
sleep 0.3

# --- 027-3: v 選択 + y ヤンク ---
./tools/test-utils/send-keys.sh --pid "$SDIT_PID" "echo 'copy this text'"
osascript -e 'tell application "System Events" to keystroke return'
sleep 1

osascript -e 'tell application "System Events" to keystroke "v" using {command down, shift down}'
sleep 0.3
# 行頭に移動
osascript -e 'tell application "System Events" to keystroke "0"'
sleep 0.1
# v で文字選択開始
osascript -e 'tell application "System Events" to keystroke "v"'
sleep 0.1
# l で 4 文字分選択
for _ in $(seq 1 4); do
    osascript -e 'tell application "System Events" to keystroke "l"'
    sleep 0.05
done
sleep 0.5
./tools/test-utils/capture-window --pid "$SDIT_PID" tmp/027-selection.png

# y でヤンク
osascript -e 'tell application "System Events" to keystroke "y"'
sleep 0.5
./tools/test-utils/capture-window --pid "$SDIT_PID" tmp/027-after-yank.png

# pbpaste で確認
./tools/test-utils/send-keys.sh --pid "$SDIT_PID" "pbpaste"
osascript -e 'tell application "System Events" to keystroke return'
sleep 1
./tools/test-utils/capture-window --pid "$SDIT_PID" tmp/027-pbpaste.png

# --- 027-6: vi カーソル描画確認 ---
# (現在のウィンドウを使って通常カーソルを先にキャプチャ)
./tools/test-utils/capture-window --pid "$SDIT_PID" tmp/027-normal-cursor.png

osascript -e 'tell application "System Events" to keystroke "v" using {command down, shift down}'
sleep 0.5
./tools/test-utils/capture-window --pid "$SDIT_PID" tmp/027-vi-cursor.png

osascript -e 'tell application "System Events" to key code 53'
sleep 0.3

# クラッシュチェック
./tools/test-utils/window-info --pid "$SDIT_PID" >/dev/null

# 結果確認
FAIL=0
for f in tmp/027-base.png tmp/027-vi-active.png tmp/027-vi-exit.png \
          tmp/027-hjkl-start.png tmp/027-hjkl-up.png tmp/027-hjkl-moved.png \
          tmp/027-selection.png tmp/027-after-yank.png tmp/027-pbpaste.png \
          tmp/027-normal-cursor.png tmp/027-vi-cursor.png; do
    if [ ! -f "$f" ]; then
        echo "FAIL: $f not created"
        FAIL=1
        continue
    fi
    SIZE=$(wc -c < "$f")
    if [ "$SIZE" -lt 10240 ]; then
        echo "FAIL: $f is too small ($SIZE bytes — possibly black screen)"
        FAIL=1
    else
        echo "OK: $f ($SIZE bytes)"
    fi
done

if [ $FAIL -eq 0 ]; then
    echo "All 027 automated checks passed."
    echo "NOTE: Cursor shape difference, selection highlight, and search result require manual visual inspection."
fi
```

## ユニットテスト対応

以下の単体テストが実装済み（`cargo test` で検証可能）:

- `vi_cursor_move_down` — j モーション
- `vi_cursor_move_up` — k モーション
- `vi_cursor_move_left` — h モーション
- `vi_cursor_move_right` — l モーション
- `vi_cursor_clamp_at_bottom` — 下端クランプ
- `vi_cursor_clamp_at_top_no_history` — 上端クランプ
- `vi_cursor_top_motion` — gg モーション
- `vi_cursor_bottom_motion` — G モーション
- `vi_cursor_word_right_basic` — w モーション
- `vi_cursor_word_left_basic` — b モーション
- `vi_cursor_first_last` — 0/$  モーション
- `vi_cursor_screen_top_middle_bottom` — H/M/L モーション

```bash
cargo test --package sdit-core terminal::vi_mode
```

## 関連

- Phase 15.1: `docs/plans/phase15.1-vi-mode.md`
- `crates/sdit/src/vi_mode.rs` — vi モードキー処理・トグル・ヤンク
- `crates/sdit-core/src/terminal/vi_mode.rs` — ViCursor・ViMotion 実装
- `crates/sdit/src/app.rs` — `ViModeState`, `ViSelectionKind`
- `crates/sdit/src/render.rs` — vi カーソル描画（ブロックカーソル）
- `crates/sdit-core/src/config/keybinds.rs` — `Action::ToggleViMode`（デフォルト: Cmd+Shift+V）

## 制限事項

- vi カーソル形状差異、選択ハイライト、検索結果ハイライトの確認は目視のみ
- ディスプレイスリープ中は GUI テストを実施できない → UNIT_ONLY として記録
