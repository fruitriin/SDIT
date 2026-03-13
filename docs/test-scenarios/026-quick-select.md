# 026: Quick Select

## 目的

Cmd+Shift+Space でキーボードショートカットによる Quick Select モードを起動し、
画面上のパターン（URL、ファイルパス、git ハッシュ、数値）をヒントキーでクリップボードにコピーできることを確認する。

## 前提条件

- `cargo build --package sdit`
- `tools/test-utils/build.sh`
- Screen Recording 権限 + Accessibility 権限
- ディスプレイがアクティブ状態（Display Asleep: No）

## 手順

### 026-1: Quick Select モード起動（Cmd+Shift+Space）

1. SDIT をバックグラウンドで起動する
2. window-info でウィンドウの存在を確認する（最大 15 秒ポーリング）
3. IME 干渉を防ぐため `key code 102`（英数キー）を送信して英語入力モードにする（delay 0.3）
4. send-keys で `echo "https://example.com /usr/local/bin abc1234 192.168.1.1"` を入力して Return キーを送信する
5. 1 秒待機してベースライン画像をキャプチャする（`tmp/026-base.png`）
6. Cmd+Shift+Space を送信して Quick Select モードを起動する
7. 1 秒待機してスクリーンショットをキャプチャする（`tmp/026-qs-active.png`）
8. ベースラインとの差分確認（ヒントラベルが表示されている）

### 026-2: URL パターンマッチとコピー

1. （026-1 の継続、Quick Select モードが起動している状態）
2. URL のヒントラベル（通常 "a"）を入力する
3. 0.5 秒待機後スクリーンショットをキャプチャする（`tmp/026-url-copy.png`）
4. Quick Select モードが終了している（ヒントラベルが消えている）ことを確認する
5. send-keys で `pbpaste` を入力して Return キーを送信する
6. 1 秒待機後スクリーンショットをキャプチャする（`tmp/026-url-paste.png`）
7. キャプチャに "https://example.com" が含まれていることを目視確認する

### 026-3: ファイルパスパターンマッチ

1. SDIT を再起動する（または新しいウィンドウで継続）
2. send-keys で `echo /usr/local/bin/fish` を入力して Return キーを送信する
3. 1 秒待機する
4. Cmd+Shift+Space を送信して Quick Select モードを起動する
5. 1 秒待機する
6. ファイルパスのヒントラベルを入力する
7. 0.5 秒待機後 Quick Select モードが終了していることを確認する

### 026-4: git ハッシュパターンマッチ

1. send-keys で `echo "commit abc1234def56 merged"` を入力して Return キーを送信する
2. 1 秒待機する
3. Cmd+Shift+Space を送信して Quick Select モードを起動する
4. 1 秒待機してスクリーンショットをキャプチャする（`tmp/026-hash-qs.png`）
5. git ハッシュのヒントラベルを入力する
6. 0.5 秒待機後 Quick Select モードが終了していることを確認する

### 026-5: Escape でキャンセル

1. send-keys で `echo https://cancel.example.com` を入力して Return キーを送信する
2. 1 秒待機する
3. Cmd+Shift+Space を送信して Quick Select モードを起動する
4. 1 秒待機してスクリーンショットをキャプチャする（`tmp/026-before-cancel.png`）
5. Escape キーを送信する
6. 0.5 秒待機後スクリーンショットをキャプチャする（`tmp/026-after-cancel.png`）
7. Quick Select モードが終了している（ヒントラベルが消えている）ことを確認する
8. SDIT プロセスがクラッシュしていないことを window-info で確認する

### 026-6: パターンがない場合にモードが起動しない

1. send-keys で `echo "no patterns here"` を入力して Return キーを送信する（URLなし・パスなし・ハッシュなし）
2. 1 秒待機する
3. Cmd+Shift+Space を送信する
4. 1 秒待機してスクリーンショットをキャプチャする（`tmp/026-no-patterns.png`）
5. ヒントラベルが表示されていないことを確認する（モードが起動しない）
6. SDIT プロセスがクラッシュしていないことを window-info で確認する

### 026-7: カスタムパターン設定

1. 設定ファイル（`~/.config/sdit/sdit.toml` または `~/Library/Application Support/sdit/sdit.toml`）に以下を追加する:
   ```toml
   [quick_select]
   patterns = ["FOO-\\d+"]
   ```
2. SDIT を再起動するか設定ホットリロードを待機する
3. send-keys で `echo "issue FOO-123 is fixed"` を入力して Return キーを送信する
4. 1 秒待機する
5. Cmd+Shift+Space を送信して Quick Select モードを起動する
6. 1 秒待機してスクリーンショットをキャプチャする（`tmp/026-custom-pattern.png`）
7. "FOO-123" のヒントラベルが表示されていることを確認する（目視）
8. 設定を元に戻す

## 期待結果

### 026-1
- `tmp/026-base.png` のファイルサイズが 10 KiB 以上
- `tmp/026-qs-active.png` でヒントラベル（"a"、"s" など）が表示されている（目視）
- SDIT プロセスがクラッシュしていない

### 026-2
- ヒントラベル入力後に Quick Select モードが終了する
- クリップボードに URL が入っている（`pbpaste` で確認）

### 026-3
- ファイルパスがパターンとして検出される

### 026-4
- git ハッシュがパターンとして検出される
- `tmp/026-hash-qs.png` にヒントラベルが表示されている

### 026-5
- Escape でモードが終了する
- `tmp/026-after-cancel.png` でヒントラベルが消えている

### 026-6
- パターンなし時は Quick Select モードが起動しない（SDIT ログに "QuickSelect: no patterns found" が出力される）
- 通常の入力状態が維持される

### 026-7
- カスタムパターンがデフォルトパターンに追加される
- "FOO-123" が検出されヒントラベルが表示される

## クリーンアップ

- SDIT プロセスを終了する
- `tmp/026-*.png` を削除する
- カスタムパターン設定を元に戻す

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
    rm -f tmp/026-*.png
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

# --- 026-1: Quick Select モード起動 ---
./tools/test-utils/send-keys.sh --pid "$SDIT_PID" "echo 'https://example.com /usr/local/bin abc1234 192.168.1.1'"
osascript -e 'tell application "System Events" to keystroke return'
sleep 1
./tools/test-utils/capture-window --pid "$SDIT_PID" tmp/026-base.png

# Cmd+Shift+Space で Quick Select 起動
osascript -e 'tell application "System Events" to keystroke space using {command down, shift down}'
sleep 1
./tools/test-utils/capture-window --pid "$SDIT_PID" tmp/026-qs-active.png

# --- 026-5: Escape でキャンセル ---
osascript -e 'tell application "System Events" to key code 53'  # Escape
sleep 0.5
./tools/test-utils/capture-window --pid "$SDIT_PID" tmp/026-after-cancel.png

# --- 026-6: パターンなし ---
./tools/test-utils/send-keys.sh --pid "$SDIT_PID" "echo 'no patterns here'"
osascript -e 'tell application "System Events" to keystroke return'
sleep 1
osascript -e 'tell application "System Events" to keystroke space using {command down, shift down}'
sleep 1
./tools/test-utils/capture-window --pid "$SDIT_PID" tmp/026-no-patterns.png

# クラッシュチェック
./tools/test-utils/window-info --pid "$SDIT_PID" >/dev/null

# 結果確認
FAIL=0
for f in tmp/026-base.png tmp/026-qs-active.png tmp/026-after-cancel.png tmp/026-no-patterns.png; do
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
    echo "All 026 automated checks passed."
    echo "NOTE: Hint label verification requires manual visual inspection."
fi
```

## ユニットテスト対応

以下の単体テストが実装済み（`cargo test` で検証可能）:

- `detect_patterns_finds_file_paths` — ファイルパス検出
- `detect_patterns_finds_git_hashes` — git ハッシュ検出
- `detect_patterns_finds_urls_and_paths` — URL + パス混在
- `detect_patterns_no_overlap` — URL 内のパスが重複検出されない
- `detect_patterns_custom_patterns` — カスタムパターン動作
- `detect_patterns_sorted_by_col` — 列順ソート

```bash
cargo test --package sdit-core terminal::url_detector
```

## 関連

- Phase 14.6: `docs/plans/phase14.6-quick-select.md`
- `crates/sdit-core/src/terminal/url_detector.rs` — `detect_patterns_in_line`, `default_quick_select_patterns`
- `crates/sdit/src/quick_select.rs` — QuickSelect モードのキー処理・モード起動
- `crates/sdit/src/app.rs` — `QuickSelectState`, `QuickSelectHint`, `generate_label`
- `crates/sdit-core/src/config/mod.rs` — `QuickSelectConfig`
- `crates/sdit-core/src/config/keybinds.rs` — `Action::QuickSelect`, デフォルトキーバインド（Cmd+Shift+Space）

## 制限事項

- ヒントラベル文字の確認は目視のみ（ピクセル単位の自動検証は困難）
- ディスプレイスリープ中は GUI テストを実施できない → UNIT_ONLY として記録
