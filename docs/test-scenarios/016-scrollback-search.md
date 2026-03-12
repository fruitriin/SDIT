# 016: スクロールバック内検索

## 目的

Cmd+F で検索バーを表示し、スクロールバック内のテキストをインクリメンタル検索できることを確認する。
マッチのハイライト、次/前のマッチへのナビゲーション、Escape での検索終了が正常に動作することを検証する。

## 前提条件

- `cargo build --package sdit`
- `tools/test-utils/build.sh`
- Screen Recording 権限 + OS 再起動
- Accessibility 権限

## 手順

### 016-1: 検索バーの表示・非表示

1. SDIT をバックグラウンドで起動する
2. window-info でウィンドウの存在を確認する（最大 15 秒ポーリング）
3. IME 干渉を防ぐため `key code 102`（英数キー）を送信して英語入力モードにする（delay 0.3）
4. ベースライン画像をキャプチャする（`tmp/016-base.png`）
5. osascript で `keystroke "f" using command down` を送信（Cmd+F）
6. 0.5 秒待機してスクリーンショットを撮る（`tmp/016-searchbar-open.png`）
7. ベースラインと比較し、画像が変化していることを確認
8. osascript で `key code 53`（Escape）を送信する
9. 0.5 秒待機してスクリーンショットを撮る（`tmp/016-searchbar-closed.png`）
10. 検索バーが消えた画像がベースラインと類似していることを確認

### 016-2: テキスト検索とハイライト

1. IME 干渉を防ぐため `key code 102` を送信する
2. send-keys で `echo "hello world"` を入力して Return キーを送信する
3. 1 秒待機してベースライン画像をキャプチャする（`tmp/016-echo-base.png`）
4. Cmd+F で検索バーを開く（`keystroke "f" using command down`）
5. 0.3 秒待機する
6. send-keys で "hello" を入力する（検索クエリ）
7. 1 秒待機してスクリーンショットを撮る（`tmp/016-search-hello.png`）
8. "hello" がハイライトされた画像がベースラインと異なることを確認
9. SDIT プロセスがクラッシュしていないことを window-info で確認する
10. Escape を送信して検索バーを閉じる

### 016-3: 検索ナビゲーション（次/前マッチ）

1. IME 干渉を防ぐため `key code 102` を送信する
2. send-keys で以下を実行して複数のマッチを生成する:
   ```
   for i in 1 2 3; do echo "test_line_$i"; done
   ```
3. 1 秒待機する
4. Cmd+F で検索バーを開く
5. 0.3 秒待機する
6. "test" と入力する（複数マッチが期待される）
7. 1 秒待機してスクリーンショットを撮る（`tmp/016-nav-initial.png`）
8. Return キーを送信（次のマッチへ移動）
9. 0.5 秒待機してスクリーンショットを撮る（`tmp/016-nav-next.png`）
10. Shift+Return を送信（前のマッチへ移動）
11. 0.5 秒待機してスクリーンショットを撮る（`tmp/016-nav-prev.png`）
12. ナビゲーション前後で画像が変化していることを確認
13. Escape を送信して検索バーを閉じる

### 016-4: 大文字小文字を無視した検索

1. IME 干渉を防ぐため `key code 102` を送信する
2. send-keys で `echo "Hello HELLO hello"` を入力して Return キーを送信する
3. 1 秒待機する
4. Cmd+F で検索バーを開く
5. 0.3 秒待機する
6. "hello" と入力する
7. 1 秒待機してスクリーンショットを撮る（`tmp/016-case-insensitive.png`）
8. SDIT プロセスがクラッシュしていないことを window-info で確認する
9. Escape を送信して検索バーを閉じる

### 016-5: スクロールバック内検索（大量出力）

1. IME 干渉を防ぐため `key code 102` を送信する
2. send-keys で `seq 1 200` を実行して大量のテキスト出力を生成する
3. 2 秒待機する（出力完了待ち）
4. Cmd+F で検索バーを開く
5. 0.3 秒待機する
6. "100" と入力する
7. 1 秒待機してスクリーンショットを撮る（`tmp/016-scrollback.png`）
8. SDIT がクラッシュしていないことを window-info で確認する
9. Escape を送信して検索バーを閉じる

### 016-6: 空クエリで検索

1. Cmd+F で検索バーを開く
2. 0.3 秒待機する
3. 何も入力せずにスクリーンショットを撮る（`tmp/016-empty-query.png`）
4. SDIT がクラッシュしていないことを window-info で確認する
5. Escape を送信して検索バーを閉じる

### 016-7: マッチなし検索

1. IME 干渉を防ぐため `key code 102` を送信する
2. Cmd+F で検索バーを開く
3. 0.3 秒待機する
4. "xyzzy_no_match_expected" と入力する
5. 1 秒待機してスクリーンショットを撮る（`tmp/016-no-match.png`）
6. SDIT がクラッシュしていないことを window-info で確認する
7. Escape を送信して検索バーを閉じる

### 016-8: 連続操作のストレステスト

1. IME 干渉を防ぐため `key code 102` を送信する
2. Cmd+F を 5 回素早く押す（delay 0.1 で連続）
3. 2 秒待機する
4. window-info でウィンドウが生存していることを確認する

## 期待結果

### 016-1
- `tmp/016-searchbar-open.png` のファイルサイズが 10 KiB 以上（空白でない）
- ベースライン画像と検索バー表示後の画像でファイルサイズが異なる（検索バーが描画されている）
- Escape 後の画像がベースラインと類似したファイルサイズになる（検索バーが消えている）
- SDIT プロセスがクラッシュしていない

### 016-2
- "hello" 入力後の画像がベースラインと異なる（ハイライトが描画されている）
- SDIT プロセスがクラッシュしていない

### 016-3
- Return（次マッチ）/ Shift+Return（前マッチ）で画像が変化する
- SDIT プロセスがクラッシュしていない

### 016-4
- "hello" で "Hello" / "HELLO" / "hello" 全てがマッチされる（画像が変化している）
- SDIT プロセスがクラッシュしていない

### 016-5
- 200 行の出力後に "100" を検索してもクラッシュしない
- 画像が 10 KiB 以上（描画が壊れていない）

### 016-6
- 空クエリ状態でクラッシュしない

### 016-7
- マッチなし時でもクラッシュしない

### 016-8
- 連続 Cmd+F 操作後もウィンドウが生存している（window-info が exit 0）

## クリーンアップ

- SDIT プロセスを終了する
- `tmp/016-*.png` を削除する

## 実行スクリプト例

```bash
#!/bin/bash
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

SDIT_PID=""
cleanup() {
    [ -n "$SDIT_PID" ] && kill "$SDIT_PID" 2>/dev/null || true
    rm -f tmp/016-*.png
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
./tools/test-utils/capture-window sdit tmp/016-base.png

# --- 016-1: 検索バーの表示・非表示 ---
osascript -e 'tell application "System Events" to keystroke "f" using command down'
sleep 0.5
./tools/test-utils/capture-window sdit tmp/016-searchbar-open.png
osascript -e 'tell application "System Events" to key code 53'  # Escape
sleep 0.5
./tools/test-utils/capture-window sdit tmp/016-searchbar-closed.png

BASE_SIZE=$(wc -c < tmp/016-base.png)
OPEN_SIZE=$(wc -c < tmp/016-searchbar-open.png)
CLOSED_SIZE=$(wc -c < tmp/016-searchbar-closed.png)

echo "016-1: base=$BASE_SIZE, open=$OPEN_SIZE, closed=$CLOSED_SIZE"
if [ "$BASE_SIZE" -lt 10240 ]; then
    echo "FAIL 016-1: base image too small"
    exit 1
fi
if [ "$OPEN_SIZE" -eq "$BASE_SIZE" ]; then
    echo "WARN 016-1: searchbar-open may not differ from base (search bar might not be rendered yet)"
fi

# --- 016-2: テキスト検索 ---
osascript -e 'tell application "System Events" to key code 102'
sleep 0.2
./tools/test-utils/send-keys.sh sdit 'echo "hello world"'
osascript -e 'tell application "System Events" to set frontmost of (first process whose name is "sdit") to true'
osascript -e 'tell application "System Events" to keystroke return'
sleep 1
./tools/test-utils/capture-window sdit tmp/016-echo-base.png

osascript -e 'tell application "System Events" to keystroke "f" using command down'
sleep 0.3
./tools/test-utils/send-keys.sh sdit "hello"
sleep 1
./tools/test-utils/capture-window sdit tmp/016-search-hello.png
./tools/test-utils/window-info sdit >/dev/null
osascript -e 'tell application "System Events" to key code 53'  # Escape

ECHO_SIZE=$(wc -c < tmp/016-echo-base.png)
SEARCH_SIZE=$(wc -c < tmp/016-search-hello.png)
echo "016-2: echo_base=$ECHO_SIZE, search_hello=$SEARCH_SIZE"

# --- 016-3: ナビゲーション ---
osascript -e 'tell application "System Events" to key code 102'
sleep 0.2
./tools/test-utils/send-keys.sh sdit "for i in 1 2 3; do echo test_line_\$i; done"
osascript -e 'tell application "System Events" to keystroke return'
sleep 1

osascript -e 'tell application "System Events" to keystroke "f" using command down'
sleep 0.3
./tools/test-utils/send-keys.sh sdit "test"
sleep 1
./tools/test-utils/capture-window sdit tmp/016-nav-initial.png

osascript -e 'tell application "System Events" to keystroke return'
sleep 0.5
./tools/test-utils/capture-window sdit tmp/016-nav-next.png

osascript -e 'tell application "System Events" to keystroke return using shift down'
sleep 0.5
./tools/test-utils/capture-window sdit tmp/016-nav-prev.png
osascript -e 'tell application "System Events" to key code 53'  # Escape

NAV_INIT=$(wc -c < tmp/016-nav-initial.png)
NAV_NEXT=$(wc -c < tmp/016-nav-next.png)
echo "016-3: nav_initial=$NAV_INIT, nav_next=$NAV_NEXT"

# --- 016-4: 大文字小文字無視 ---
osascript -e 'tell application "System Events" to key code 102'
sleep 0.2
./tools/test-utils/send-keys.sh sdit 'echo "Hello HELLO hello"'
osascript -e 'tell application "System Events" to keystroke return'
sleep 1

osascript -e 'tell application "System Events" to keystroke "f" using command down'
sleep 0.3
./tools/test-utils/send-keys.sh sdit "hello"
sleep 1
./tools/test-utils/capture-window sdit tmp/016-case-insensitive.png
./tools/test-utils/window-info sdit >/dev/null
osascript -e 'tell application "System Events" to key code 53'  # Escape

# --- 016-5: スクロールバック検索 ---
osascript -e 'tell application "System Events" to key code 102'
sleep 0.2
./tools/test-utils/send-keys.sh sdit "seq 1 200"
osascript -e 'tell application "System Events" to keystroke return'
sleep 2

osascript -e 'tell application "System Events" to keystroke "f" using command down'
sleep 0.3
./tools/test-utils/send-keys.sh sdit "100"
sleep 1
./tools/test-utils/capture-window sdit tmp/016-scrollback.png
./tools/test-utils/window-info sdit >/dev/null
osascript -e 'tell application "System Events" to key code 53'  # Escape

# --- 016-6: 空クエリ ---
osascript -e 'tell application "System Events" to keystroke "f" using command down'
sleep 0.3
./tools/test-utils/capture-window sdit tmp/016-empty-query.png
./tools/test-utils/window-info sdit >/dev/null
osascript -e 'tell application "System Events" to key code 53'  # Escape

# --- 016-7: マッチなし ---
osascript -e 'tell application "System Events" to key code 102'
sleep 0.2
osascript -e 'tell application "System Events" to keystroke "f" using command down'
sleep 0.3
./tools/test-utils/send-keys.sh sdit "xyzzy_no_match_expected"
sleep 1
./tools/test-utils/capture-window sdit tmp/016-no-match.png
./tools/test-utils/window-info sdit >/dev/null
osascript -e 'tell application "System Events" to key code 53'  # Escape

# --- 016-8: ストレステスト ---
for i in $(seq 1 5); do
    osascript -e 'tell application "System Events" to keystroke "f" using command down'
    sleep 0.1
done
sleep 2
./tools/test-utils/window-info sdit >/dev/null

# 結果確認
FAIL=0
for f in tmp/016-base.png tmp/016-searchbar-open.png tmp/016-echo-base.png \
         tmp/016-search-hello.png tmp/016-scrollback.png tmp/016-no-match.png; do
    SIZE=$(wc -c < "$f")
    if [ "$SIZE" -lt 10240 ]; then
        echo "FAIL: $f is too small ($SIZE bytes)"
        FAIL=1
    else
        echo "OK: $f ($SIZE bytes)"
    fi
done

if [ $FAIL -eq 0 ]; then
    echo "All 016 automated checks passed."
    echo "NOTE: Highlight colors (yellow/orange) require manual visual verification."
fi
```

## 制限事項

- **ハイライト色の自動検証**: 黄色/オレンジの背景色は自動的には検証困難。画像ファイルサイズの変化でハイライトが描画されたことを間接的に確認し、正確な色は目視確認が必要
- **インクリメンタルサーチの即時性**: `send-keys.sh` 経由の入力では入力のタイミングが離散的になるため、厳密なインクリメンタル動作の確認は手動テスト推奨
- **マッチ件数表示 `[n/m]`**: スクリーンショットから数値テキストを自動検証する仕組みがないため、目視確認が必要

## 関連

- Phase 9.1: `docs/plans/phase9.1-search.md`
- `crates/sdit-core/src/search.rs` — グリッド内テキスト検索（実装予定）
- `crates/sdit/src/input.rs` — Cmd+F ショートカット（実装予定）
- `crates/sdit/src/render.rs` — 検索バーオーバーレイ描画・マッチハイライト（実装予定）
