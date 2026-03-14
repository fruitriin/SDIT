# muda メニューバー統合の知見（Phase 21.5）

## 重要: init_for_nsapp() の呼び出しタイミング

### 問題

`main.rs` で `menu_bar.init_for_nsapp()` を `event_loop.run_app()` の **前** に呼ぶと、
winit が `run_app()` 内で NSApplication をセットアップする際にメニューバーをデフォルトで上書きするため、
独自メニューが消える。

### 解決策

`init_for_nsapp()` を winit の `resumed()` コールバック内（`run_app()` の後）で呼ぶ。

```rust
// event_loop.rs
fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
    // init_for_nsapp() を event_loop.run_app() より前に呼ぶと NSApp 初期化で上書きされる。
    // resumed() 内で1回だけ呼ぶことで回避する。
    if let Some(menu_bar) = self.menu_bar.take() {
        menu_bar.init_for_nsapp();
    }
    // ... ウィンドウ生成
}
```

`menu_bar` を `Option<Menu>` で持ち、`take()` で一度だけ実行するパターンを使う。

### muda メニュー構成（Phase 21.5 時点）

macOS ネイティブメニューバーは以下の構成:
- **sdit**: About SDIT, Settings…, Quit SDIT
- **File**: New Window, New Tab, Close Tab
- **Edit**: Copy, Paste, Select All
- **View**: Toggle Sidebar, Zoom In, Zoom Out, Actual Size, Search…
- **Session**: Next Tab, Previous Tab, Move Tab to New Window

## 既知の問題: muda 0.17.1 メニュークリック時クラッシュ

### 症状

メニュー項目をクリックすると SDIT がクラッシュする:

```
thread 'main' panicked at muda-0.17.1/src/platform_impl/macos/icon.rs:34:53:
called `Result::unwrap()` on an `Err` value: Format(FormatError { inner: ZeroWidth })
```

### 原因

`MenuItem::fire_menu_item_click()` 内で `PlatformIcon::to_png()` が呼ばれ、
アイコン未設定時のゼロ幅 PlatformIcon を PNG エンコードしようとして `unwrap()` がパニックする。

### 対応

`docs/plans/phase21.6-menu-click-crash.md` として独立計画を起こして対応予定。
muda のバージョンアップまたはアイコン設定で解決できる見込み。

## list-menus.sh の joinList 問題

`tools/test-utils/list-menus.sh` の AppleScript 内で `joinList` ハンドラを
`tell application "System Events"` ブロック内から呼ぶと機能せず、空リストを返す。

### 回避策

手動ループで結合する（list-menus.sh 内の AppleScript を手動結合に置き換える）:

```applescript
set jsonResult to ""
repeat with i from 1 to count of jsonParts
    if i > 1 then set jsonResult to jsonResult & ", "
    set jsonResult to jsonResult & item i of jsonParts
end repeat
```

`Feedback.md` に改善アクションとして記録済み。
