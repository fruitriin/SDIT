# muda メニューバー統合の知見（Phase 21.5 / 21.6）

## 重要: init_for_nsapp() の呼び出しタイミングと Menu の生存期間

### 問題

`main.rs` で `menu_bar.init_for_nsapp()` を `event_loop.run_app()` の **前** に呼ぶと、
winit が `run_app()` 内で NSApplication をセットアップする際にメニューバーをデフォルトで上書きするため、
独自メニューが消える。

### 解決策

`init_for_nsapp()` を winit の `resumed()` コールバック内（`run_app()` の後）で呼ぶ。

```rust
// event_loop.rs
fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
    // as_ref() を使い Menu を保持し続ける（take()→drop すると ivars が dangling になりクラッシュ）。
    #[cfg(target_os = "macos")]
    if let Some(menu_bar) = self.menu_bar.as_ref() {
        menu_bar.init_for_nsapp();
    }
    // ... ウィンドウ生成
}
```

### Phase 21.6 教訓: take() vs as_ref()

**`take()` は使ってはいけない。** `take()` で Menu を取り出すと、`resumed()` のスコープ終了時に
Menu が drop され、Objective-C 側の NSMenu/NSMenuItem の ivars が dangling ポインタになる。
その後メニューをクリックすると `fire_menu_item_click()` が解放済みメモリにアクセスしてクラッシュする。

`as_ref()` を使えば Menu は `SditApp` の `Option<Menu>` フィールドに保持され続け、
アプリケーション終了まで生存する。

### muda メニュー構成（Phase 21.5 時点）

macOS ネイティブメニューバーは以下の構成:
- **sdit**: About SDIT, Settings…, Quit SDIT
- **File**: New Window, New Tab, Close Tab
- **Edit**: Copy, Paste, Select All
- **View**: Toggle Sidebar, Zoom In, Zoom Out, Actual Size, Search…
- **Session**: Next Tab, Previous Tab, Move Tab to New Window

## 解決済み: muda 0.17.1 メニュークリック時クラッシュ（Phase 21.6）

### 原因 1: Menu の dangling ポインタ
`menu_bar.take()` で Menu を取り出すと drop され、Objective-C 側の参照が無効になる。
→ **修正**: `menu_bar.as_ref()` に変更して Menu を保持し続ける。

### 原因 2: ゼロ幅アイコンの unwrap パニック
`MenuItem::fire_menu_item_click()` 内で `PlatformIcon::to_png()` が呼ばれ、
アイコン未設定時のゼロ幅 PlatformIcon を PNG エンコードしようとして `unwrap()` がパニックする。
→ **修正**: `vendor/muda-0.17.1` にゼロ幅アイコンのガードを追加（`Cargo.toml` の `[patch.crates-io]` で参照）。

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
