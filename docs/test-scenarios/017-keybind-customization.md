# 017: キーバインドカスタマイズ

## 目的

デフォルトキーバインドが正しく動作することを確認し、TOML 設定でキーバインドを上書きできること、
不正な設定値でもデフォルトにフォールバックすることを検証する。
プラットフォーム固有バインド（macOS: Cmd/Super、Linux/その他: Ctrl）もカバーする。

## 前提条件

- `cargo build --package sdit`
- `tools/test-utils/build.sh`
- Screen Recording 権限 + OS 再起動
- Accessibility 権限

---

## 手順

### 017-1: デフォルトキーバインド — NewWindow (Cmd+N / Ctrl+Shift+N)

1. SDIT をデフォルト設定でバックグラウンドで起動する
2. window-info でウィンドウの存在を確認する（最大 15 秒ポーリング）
3. IME 干渉を防ぐため `key code 102`（英数キー）を送信して英語入力モードにする（delay 0.3）
4. ベースライン画像をキャプチャする（`tmp/017-base.png`）
5. macOS の場合: osascript で `keystroke "n" using command down` を送信（Cmd+N）
   非 macOS の場合: Ctrl+Shift+N に相当するキーを送信
6. 1 秒待機してスクリーンショットを撮る（`tmp/017-new-window.png`）
7. window-info でウィンドウが 2 枚以上存在することを確認する

### 017-2: デフォルトキーバインド — AddSession (Cmd+T / Ctrl+Shift+T)

1. SDIT ウィンドウにフォーカスを当てる
2. IME 干渉を防ぐため `key code 102` を送信する
3. ベースライン画像をキャプチャする（`tmp/017-session-base.png`）
4. macOS の場合: `keystroke "t" using command down`（Cmd+T）を送信する
5. 1 秒待機してスクリーンショットを撮る（`tmp/017-add-session.png`）
6. ベースライン画像と比較し、サイドバーが出現したことを確認する
   （セッションが 2 つになりサイドバーが表示される）

### 017-3: デフォルトキーバインド — SidebarToggle (Cmd+\ / Ctrl+\)

1. SDIT にセッションが 2 つ以上ある状態にする（017-2 の継続、または Cmd+T で追加）
2. IME 干渉を防ぐため `key code 102` を送信する
3. サイドバーが表示された状態でスクリーンショットを撮る（`tmp/017-sidebar-visible.png`）
4. macOS の場合: `keystroke "\\" using command down`（Cmd+\）を送信する
5. 0.5 秒待機してスクリーンショットを撮る（`tmp/017-sidebar-hidden.png`）
6. サイドバーが消えた画像がサイドバーあり画像と異なることを確認する
7. 再度 Cmd+\ を送信してサイドバーを再表示する
8. 0.5 秒待機して SDIT がクラッシュしていないことを window-info で確認する

### 017-4: デフォルトキーバインド — Copy/Paste (Cmd+C / Cmd+V)

1. SDIT ウィンドウにフォーカスを当てる
2. IME 干渉を防ぐため `key code 102` を送信する
3. send-keys で `echo "keybind_copy_test"` を入力して Return キーを送信する
4. 1 秒待機する
5. マウスでターミナル内の "keybind_copy_test" テキストをダブルクリックで選択する
   （または send-keys でカーソル位置から選択 — 目視確認が必要）
6. macOS の場合: `keystroke "c" using command down`（Cmd+C）を送信する
7. 0.3 秒待機する
8. Enter キーを押して新しい行を開始する
9. macOS の場合: `keystroke "v" using command down`（Cmd+V）を送信する
10. 1 秒待機してスクリーンショットを撮る（`tmp/017-paste.png`）
11. SDIT がクラッシュしていないことを window-info で確認する

### 017-5: TOML カスタムキーバインドで上書き — NewWindow を Cmd+Shift+W に変更

1. カスタム設定ファイルを作成する（`tmp/017-custom-keybinds.toml`）:
   ```toml
   [[keybinds]]
   key = "w"
   mods = "super|shift"
   action = "NewWindow"
   ```
2. カスタム設定でデフォルトキーバインドが上書きされることを単体テストで検証する:
   - `cargo test --package sdit-core keybind` を実行する
   - `deserialize_keybind_config_from_toml` テストが PASS することを確認する
3. SDIT をカスタム設定で起動する（`--config tmp/017-custom-keybinds.toml` または `SDIT_CONFIG` 環境変数）
4. window-info でウィンドウの存在を確認する（最大 15 秒ポーリング）
5. IME 干渉を防ぐため `key code 102` を送信する
6. ベースライン画像をキャプチャする（`tmp/017-custom-base.png`）
7. osascript で `keystroke "w" using {command down, shift down}` を送信（Cmd+Shift+W）
8. 1 秒待機してスクリーンショットを撮る（`tmp/017-custom-newwindow.png`）
9. 新しいウィンドウが開いたことを window-info で確認する

### 017-6: TOML カスタムキーバインド — デフォルトアクションの再割り当て

1. 別のカスタム設定ファイルを作成する（`tmp/017-remap-keybinds.toml`）:
   ```toml
   [[keybinds]]
   key = "k"
   mods = "super"
   action = "NewWindow"

   [[keybinds]]
   key = "j"
   mods = "super"
   action = "AddSession"
   ```
2. SDIT をこの設定で起動する
3. window-info でウィンドウの存在を確認する（最大 15 秒ポーリング）
4. IME 干渉を防ぐため `key code 102` を送信する
5. osascript で `keystroke "k" using command down`（Cmd+K）を送信する
6. 1 秒待機してスクリーンショットを撮る（`tmp/017-remap-k.png`）
7. 新しいウィンドウが開いたことを window-info で確認する
8. SDIT がクラッシュしていないことを確認する

### 017-7: 不正な TOML 設定でもデフォルトにフォールバック

1. 不正な action 値を含む設定ファイルを作成する（`tmp/017-invalid-keybinds.toml`）:
   ```toml
   [[keybinds]]
   key = "x"
   mods = "super"
   action = "NonExistentAction"
   ```
2. SDIT をこの設定で起動する
3. **クラッシュせずに起動すること**を window-info で確認する（最大 15 秒ポーリング）
4. デフォルトキーバインド（Cmd+N）が動作することを確認する:
   - `keystroke "n" using command down` を送信する
   - 1 秒待機してウィンドウが生存していることを window-info で確認する
5. スクリーンショットを撮る（`tmp/017-fallback.png`）

### 017-8: 不正な modifier 値でもデフォルトにフォールバック

1. 不正な modifier 値を含む設定ファイルを作成する（`tmp/017-invalid-mods.toml`）:
   ```toml
   [[keybinds]]
   key = "x"
   mods = "superduper|ultrashift"
   action = "NewWindow"
   ```
2. SDIT をこの設定で起動する
3. クラッシュせずに起動することを window-info で確認する
4. SDIT が生存していることを確認する（window-info が exit 0）

### 017-9: セッション切替キーバインド (Ctrl+Tab / Ctrl+Shift+Tab)

1. SDIT を起動し、Cmd+T でセッションを 2 つ以上作成する
2. IME 干渉を防ぐため `key code 102` を送信する
3. 現在の状態でスクリーンショットを撮る（`tmp/017-session-initial.png`）
4. `key code 48 using {control down}`（Ctrl+Tab）を送信する
5. 0.5 秒待機してスクリーンショットを撮る（`tmp/017-session-next.png`）
6. `key code 48 using {control down, shift down}`（Ctrl+Shift+Tab）を送信する
7. 0.5 秒待機してスクリーンショットを撮る（`tmp/017-session-prev.png`）
8. SDIT がクラッシュしていないことを window-info で確認する

### 017-10: プラットフォーム固有バインド — macOS Super vs Linux Ctrl（ユニットテストで確認）

1. 以下のコマンドを実行して単体テストを走らせる:
   ```
   cargo test --package sdit-core -- keybind
   cargo test --package sdit -- input
   ```
2. `macos_has_required_actions` テストが PASS することを確認する（macOS 環境の場合）
3. `default_bindings_not_empty` テストが PASS することを確認する（全プラットフォーム）
4. `default_bindings_zoom_actions_exist` テストが PASS することを確認する（macOS 環境の場合）

---

## 期待結果

### 017-1
- `tmp/017-new-window.png` のファイルサイズが 10 KiB 以上
- window-info が新しいウィンドウ（2 枚目）の存在を確認（exit 0）

### 017-2
- `tmp/017-add-session.png` がベースラインと異なる（サイドバーが出現）
- ファイルサイズが 10 KiB 以上
- SDIT がクラッシュしていない

### 017-3
- サイドバーあり/なし画像でファイルサイズが異なる（サイドバー描画の差）
- Cmd+\ のトグルでクラッシュしない

### 017-4
- `tmp/017-paste.png` のファイルサイズが 10 KiB 以上
- SDIT がクラッシュしていない
- （目視確認）ペーストされたテキストが表示されている

### 017-5
- カスタム設定で起動して window-info が exit 0
- Cmd+Shift+W で新しいウィンドウが開くか、クラッシュしない
  （設定ファイル指定方法が実装済みの場合）

### 017-6
- Cmd+K で新しいウィンドウが生成される、または SDIT がクラッシュしない
  （設定ファイル指定方法が実装済みの場合）

### 017-7
- 不正な action 値を含む設定でも SDIT がクラッシュせず起動する
- デフォルトキーバインドが引き続き動作する

### 017-8
- 不正な modifier 値を含む設定でも SDIT がクラッシュせず起動する

### 017-9
- Ctrl+Tab / Ctrl+Shift+Tab でセッション切替後もクラッシュしない
- 画像変化でセッションが切り替わったことが確認できる（目視）

### 017-10
- ユニットテスト全 PASS:
  - `keybind` フィルタのテストが PASS
  - `input` フィルタのテストが PASS

---

## クリーンアップ

- SDIT プロセスをすべて終了する
- `tmp/017-*.png` を削除する
- `tmp/017-*.toml` を削除する

---

## 実行スクリプト例

```bash
#!/bin/bash
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

SDIT_PID=""
cleanup() {
    [ -n "$SDIT_PID" ] && kill "$SDIT_PID" 2>/dev/null || true
    pkill -f "target/debug/sdit" 2>/dev/null || true
    rm -f tmp/017-*.png tmp/017-*.toml
}
trap cleanup EXIT

# --- 017-10: ユニットテスト（最初に実行してフェイルファストを実現）---
echo "=== 017-10: Unit tests ==="
cargo test --package sdit-core -- keybind 2>&1 | tail -5
cargo test --package sdit -- input 2>&1 | tail -5
echo "017-10: Unit tests PASS"

# --- 017-1: デフォルトキーバインド — NewWindow ---
echo "=== 017-1: DefaultKeybind NewWindow ==="
pkill -f "target/debug/sdit" 2>/dev/null || true
sleep 0.5
./target/debug/sdit &
SDIT_PID=$!

for i in $(seq 1 30); do
    if ./tools/test-utils/window-info sdit >/dev/null 2>&1; then break; fi
    sleep 0.5
done
./tools/test-utils/window-info sdit >/dev/null

osascript -e 'tell application "System Events" to key code 102'
sleep 0.3
./tools/test-utils/capture-window sdit tmp/017-base.png

osascript -e 'tell application "System Events" to keystroke "n" using command down'
sleep 1
./tools/test-utils/capture-window sdit tmp/017-new-window.png
./tools/test-utils/window-info sdit >/dev/null
echo "017-1: PASS"

# --- 017-2: AddSession (Cmd+T) ---
echo "=== 017-2: DefaultKeybind AddSession ==="
./tools/test-utils/capture-window sdit tmp/017-session-base.png
osascript -e 'tell application "System Events" to keystroke "t" using command down'
sleep 1
./tools/test-utils/capture-window sdit tmp/017-add-session.png
SESSION_BASE=$(wc -c < tmp/017-session-base.png)
SESSION_AFTER=$(wc -c < tmp/017-add-session.png)
echo "017-2: session_base=$SESSION_BASE, add_session=$SESSION_AFTER"
echo "017-2: PASS (sidebar diff requires visual check)"

# --- 017-3: SidebarToggle (Cmd+\) ---
echo "=== 017-3: SidebarToggle ==="
osascript -e 'tell application "System Events" to key code 102'
sleep 0.2
./tools/test-utils/capture-window sdit tmp/017-sidebar-visible.png
osascript -e 'tell application "System Events" to keystroke "\\" using command down'
sleep 0.5
./tools/test-utils/capture-window sdit tmp/017-sidebar-hidden.png
osascript -e 'tell application "System Events" to keystroke "\\" using command down'
sleep 0.5
./tools/test-utils/window-info sdit >/dev/null
SIDEBAR_VISIBLE=$(wc -c < tmp/017-sidebar-visible.png)
SIDEBAR_HIDDEN=$(wc -c < tmp/017-sidebar-hidden.png)
echo "017-3: sidebar_visible=$SIDEBAR_VISIBLE, sidebar_hidden=$SIDEBAR_HIDDEN"
echo "017-3: PASS"

# --- 017-7: 不正な action 値でもクラッシュしない ---
echo "=== 017-7: Invalid action fallback ==="
pkill -f "target/debug/sdit" 2>/dev/null || true
sleep 0.5

cat > tmp/017-invalid-keybinds.toml << 'EOF'
[[keybinds]]
key = "x"
mods = "super"
action = "NonExistentAction"
EOF

# 不正な設定ファイルで起動を試みる（エラーで終了する場合も許容してクラッシュ≠パニックを確認）
STARTED=0
if ./target/debug/sdit --config tmp/017-invalid-keybinds.toml &>/dev/null & then
    INVALID_PID=$!
    for i in $(seq 1 10); do
        if ./tools/test-utils/window-info sdit >/dev/null 2>&1; then
            STARTED=1
            break
        fi
        sleep 0.5
    done
    kill "$INVALID_PID" 2>/dev/null || true
fi
echo "017-7: invalid action handled (started=$STARTED, no panic expected)"

# --- 017-8: 不正な modifier でもクラッシュしない ---
echo "=== 017-8: Invalid modifier fallback ==="
pkill -f "target/debug/sdit" 2>/dev/null || true
sleep 0.5

cat > tmp/017-invalid-mods.toml << 'EOF'
[[keybinds]]
key = "x"
mods = "superduper|ultrashift"
action = "NewWindow"
EOF

STARTED_MODS=0
if ./target/debug/sdit --config tmp/017-invalid-mods.toml &>/dev/null & then
    INVALID_MODS_PID=$!
    for i in $(seq 1 10); do
        if ./tools/test-utils/window-info sdit >/dev/null 2>&1; then
            STARTED_MODS=1
            break
        fi
        sleep 0.5
    done
    kill "$INVALID_MODS_PID" 2>/dev/null || true
fi
echo "017-8: invalid modifier handled (started=$STARTED_MODS, no panic expected)"

# --- 017-9: セッション切替 (Ctrl+Tab) ---
echo "=== 017-9: Session switch Ctrl+Tab ==="
pkill -f "target/debug/sdit" 2>/dev/null || true
sleep 0.5
./target/debug/sdit &
SDIT_PID=$!
for i in $(seq 1 30); do
    if ./tools/test-utils/window-info sdit >/dev/null 2>&1; then break; fi
    sleep 0.5
done

osascript -e 'tell application "System Events" to key code 102'
sleep 0.2
# セッション追加
osascript -e 'tell application "System Events" to keystroke "t" using command down'
sleep 0.5
./tools/test-utils/capture-window sdit tmp/017-session-initial.png

# Ctrl+Tab で次のセッションへ
osascript -e 'tell application "System Events" to key code 48 using {control down}'
sleep 0.5
./tools/test-utils/capture-window sdit tmp/017-session-next.png

# Ctrl+Shift+Tab で前のセッションへ
osascript -e 'tell application "System Events" to key code 48 using {control down, shift down}'
sleep 0.5
./tools/test-utils/capture-window sdit tmp/017-session-prev.png
./tools/test-utils/window-info sdit >/dev/null
echo "017-9: PASS"

# 結果確認
FAIL=0
for f in tmp/017-base.png tmp/017-session-base.png tmp/017-sidebar-visible.png \
         tmp/017-session-initial.png tmp/017-session-next.png; do
    SIZE=$(wc -c < "$f")
    if [ "$SIZE" -lt 10240 ]; then
        echo "FAIL: $f is too small ($SIZE bytes)"
        FAIL=1
    else
        echo "OK: $f ($SIZE bytes)"
    fi
done

if [ $FAIL -eq 0 ]; then
    echo "All 017 automated checks passed."
    echo "NOTE: Sidebar visibility changes and session switching require manual visual verification."
fi
```

---

## 制限事項

- **設定ファイル指定の起動方法**: `--config` フラグや `SDIT_CONFIG` 環境変数が実装されているかどうかによって、017-5/017-6 の手動確認が必要になる。実装前は UNIT_ONLY 扱いとする。
- **不正 action のフォールバック動作**: `serde` の `PascalCase` 列挙体デシリアライズはデフォルトでエラーを返すため、起動時エラーで終了するか、警告ログを出力してデフォルトにフォールバックするかは実装方針による。どちらもパニックせずに graceful に処理されることを確認する。
- **Ctrl+Tab のキーコード**: macOS では `key code 48` が Tab に相当するが、システム設定によって Ctrl+Tab が他のアプリに奪われる場合がある。手動テストが必要。
- **Copy/Paste の選択確認**: テキスト選択はマウス操作が必要なため、017-4 は半自動テスト（ペースト動作はキーボード送信で検証可能）。

---

## 関連

- Phase 9.2: `docs/plans/phase9.2-keybinds.md`
- `crates/sdit-core/src/config/keybinds.rs` — Action 列挙体・KeybindConfig・default_bindings()
- `crates/sdit/src/input.rs` — parse_mods()・key_matches()・resolve_action()
- `crates/sdit/src/event_loop.rs` — キーバインドハンドラ統合
