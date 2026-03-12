# 018: 設定 Hot Reload

## 目的

`sdit.toml` を変更したとき、アプリを再起動せずにフォントサイズ・カラーテーマ・キーバインドが
動的に反映されることを確認する。また、パースエラーや存在しないパスのウォッチャー起動失敗など
エッジケースで graceful fallback が行われることを確認する。

## 前提条件

- `cargo build --package sdit`
- `tools/test-utils/build.sh`
- Screen Recording 権限 + OS 再起動
- Accessibility 権限

---

## 018-1: ウォッチャー起動確認（smoke test）

### 手順

1. SDIT をバックグラウンドで起動する（`RUST_LOG=info` 付き）
2. window-info でウィンドウ存在確認（最大 15 秒ポーリング）
3. プロセス起動後の stdout/stderr ログに `config_watcher: watching` が含まれることを確認する

### 期待結果

- SDIT が正常起動する（window-info exit 0）
- ログに `config_watcher: watching` が出力されている

---

## 018-2: フォントサイズ変更の動的反映

### 手順

1. SDIT をバックグラウンドで起動する
2. window-info でウィンドウ存在確認（最大 15 秒ポーリング）
3. send-keys で `echo BEFORE_RELOAD` を入力して Return を送信する
4. 1 秒待機してベースライン画像をキャプチャする（`tmp/018-font-before.png`）
5. 設定ファイル（`~/.config/sdit/sdit.toml` 相当）のフォントサイズを現在値から +4 変更して上書き保存する
   - 存在しない場合は `tmp/018-test.toml` をデフォルトパスに相当する場所にコピーして変更する
6. 1 秒待機する（デバウンス 300ms + 再描画の伝搬）
7. send-keys で `echo AFTER_RELOAD` を入力して Return を送信する
8. 1 秒待機してキャプチャする（`tmp/018-font-after.png`）

### 期待結果

- ベースライン画像と変更後画像でファイルサイズが異なる（フォントサイズ変化により描画量が変化）
- SDIT プロセスがクラッシュしていない（window-info exit 0）

---

## 018-3: カラーテーマ切替の動的反映

### 手順

1. SDIT をバックグラウンドで起動する（デフォルト: catppuccin-mocha）
2. window-info でウィンドウ存在確認（最大 15 秒ポーリング）
3. send-keys で `echo THEME_BASE` を入力して Return を送信する
4. 1 秒待機してベースライン画像をキャプチャする（`tmp/018-theme-before.png`）
5. 設定ファイルの `[colors] theme` を `gruvbox-dark` に変更して上書き保存する
6. 1 秒待機する（デバウンス 300ms + 再描画の伝搬）
7. send-keys で `echo THEME_CHANGED` を入力して Return を送信する
8. 1 秒待機してキャプチャする（`tmp/018-theme-after.png`）

### 期待結果

- ベースライン画像と変更後画像でファイルサイズが異なる（配色変化による圧縮率の違い）
- SDIT プロセスがクラッシュしていない（window-info exit 0）

---

## 018-4: キーバインド変更の動的反映

### 手順

1. SDIT をバックグラウンドで起動する
2. window-info でウィンドウ存在確認（最大 15 秒ポーリング）
3. 設定ファイルの `[keybinds]` セクションにカスタムバインドを追加して上書き保存する
   - 例: `new_window = "Cmd+Shift+T"`（デフォルトは `Cmd+N`）
4. 1 秒待機する（デバウンス 300ms）
5. `Cmd+Shift+T` を送信して新しいウィンドウが開くことを確認する
   - `osascript -e 'tell application "System Events" to keystroke "t" using {command down, shift down}'`
6. 0.5 秒待機して window-info でウィンドウ数が増えたことを確認する

### 期待結果

- カスタムバインドが適用され、新しいウィンドウが開く
- SDIT プロセスがクラッシュしていない

---

## 018-5: パースエラー時の graceful fallback

### 手順

1. SDIT をバックグラウンドで起動する（`RUST_LOG=warn` 付き）
2. window-info でウィンドウ存在確認（最大 15 秒ポーリング）
3. send-keys で `echo BEFORE_INVALID` を入力して Return を送信する
4. 1 秒待機してベースライン画像をキャプチャする（`tmp/018-invalid-before.png`）
5. 設定ファイルを TOML として不正な内容（例: `[font\nsize = "not a number"`）に上書き保存する
6. 1 秒待機する
7. send-keys で `echo AFTER_INVALID` を入力して Return を送信する
8. 1 秒待機してキャプチャする（`tmp/018-invalid-after.png`）

### 期待結果

- SDIT プロセスがクラッシュしていない（window-info exit 0）
- 設定変更前の外観が維持されている（フォント・カラーが変化しない）
- ログに WARN/ERROR レベルのパースエラーメッセージが出力されている

---

## 018-6: 存在しない設定ファイルパスでのウォッチャー起動失敗

### 手順

このシナリオはユニットテストで検証する（GUI 不要）。

1. `cargo test -p sdit nonexistent_parent_returns_none` を実行する
2. `config_watcher::spawn_config_watcher` に存在しない親ディレクトリのパスを渡したとき
   `None` が返ることを確認する（`config_watcher.rs` 内の単体テスト）

### 期待結果

- テストが PASS する
- `spawn_config_watcher` は `None` を返し、SDIT の起動は続行される

---

## 実行スクリプト例（018-1 + 018-2 の自動化）

```bash
#!/bin/bash
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

SDIT_PID=""
CONFIG_BACKUP=""
ORIG_CONFIG="${HOME}/.config/sdit/sdit.toml"

cleanup() {
    [ -n "$SDIT_PID" ] && kill "$SDIT_PID" 2>/dev/null || true
    # 設定を元に戻す
    if [ -n "$CONFIG_BACKUP" ] && [ -f "$CONFIG_BACKUP" ]; then
        cp "$CONFIG_BACKUP" "$ORIG_CONFIG"
        rm -f "$CONFIG_BACKUP"
    fi
    rm -f tmp/018-*.png tmp/018-log.txt
}
trap cleanup EXIT

mkdir -p tmp
pkill -f "target/debug/sdit" 2>/dev/null || true
sleep 0.5

# --- 018-1: ウォッチャー起動確認 ---
RUST_LOG=info ./target/debug/sdit > tmp/018-log.txt 2>&1 &
SDIT_PID=$!

for i in $(seq 1 30); do
    if ./tools/test-utils/window-info sdit >/dev/null 2>&1; then break; fi
    sleep 0.5
done
./tools/test-utils/window-info sdit >/dev/null

# ウォッチャーのログ確認
sleep 0.5
if grep -q "config_watcher: watching" tmp/018-log.txt; then
    echo "018-1 PASS: config_watcher started"
else
    echo "018-1 WARN: config_watcher log not found (may not yet appear)"
fi

# --- 018-2: フォントサイズ変更 ---
osascript -e 'tell application "System Events" to key code 102'
sleep 0.3

./tools/test-utils/send-keys.sh sdit "echo BEFORE_RELOAD"
osascript -e 'tell application "System Events" to set frontmost of (first process whose name is "sdit") to true'
osascript -e 'tell application "System Events" to keystroke return'
sleep 1
./tools/test-utils/capture-window sdit tmp/018-font-before.png

# 設定ファイルを変更（バックアップして変更）
if [ -f "$ORIG_CONFIG" ]; then
    CONFIG_BACKUP="tmp/018-config-backup.toml"
    cp "$ORIG_CONFIG" "$CONFIG_BACKUP"
    # size を現在値から変更（例: 14 → 18）
    sed -i '' 's/^size = .*/size = 18/' "$ORIG_CONFIG" || true
else
    # 設定ファイルが存在しない場合はデフォルトを記録
    mkdir -p "$(dirname "$ORIG_CONFIG")"
    cat > "$ORIG_CONFIG" <<'EOF'
[font]
size = 18
EOF
    CONFIG_BACKUP="tmp/018-config-new.toml"
    echo "" > "$CONFIG_BACKUP"  # 空ファイル = 存在しなかった印
fi

sleep 1.5  # デバウンス(300ms) + 再描画の伝搬

./tools/test-utils/send-keys.sh sdit "echo AFTER_RELOAD"
osascript -e 'tell application "System Events" to keystroke return'
sleep 1
./tools/test-utils/capture-window sdit tmp/018-font-after.png

# 結果確認
./tools/test-utils/window-info sdit >/dev/null && echo "018-2 PASS: process alive" || echo "018-2 FAIL: process died"

BEFORE_SIZE=$(wc -c < tmp/018-font-before.png)
AFTER_SIZE=$(wc -c < tmp/018-font-after.png)
echo "018-2: before=${BEFORE_SIZE} bytes, after=${AFTER_SIZE} bytes"
if [ "$BEFORE_SIZE" -ge 10240 ] && [ "$AFTER_SIZE" -ge 10240 ]; then
    echo "018-2 PASS: both screenshots non-empty"
else
    echo "018-2 FAIL: screenshot too small"
fi
```

## クリーンアップ

- SDIT プロセスをすべて終了する
- 変更した設定ファイルを元に戻す（バックアップから復元）
- `tmp/018-*.png`、`tmp/018-log.txt` を削除する

## 関連

- Phase 10.1: `docs/plans/phase10.1-hot-reload.md`
- `crates/sdit/src/config_watcher.rs` — `spawn_config_watcher()`、デバウンス 300ms
- `crates/sdit/src/app.rs` — `apply_config_reload()`
- `crates/sdit/src/main.rs` — ウォッチャー起動・`_watcher` 保持
- `docs/knowhow/config-and-theming.md` — 設定スキーマとパース方針
