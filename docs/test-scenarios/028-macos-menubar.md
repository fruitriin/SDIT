# 028: macOS ネイティブメニューバーの確認

## 目的
macOS ネイティブメニューバー（画面左上）が正しく構成されていること、各メニュー項目が期待通りに動作することを確認する。

## 前提条件
- `cargo build --package sdit`
- `tools/test-utils/build.sh`
- Screen Recording 権限 + OS 再起動
- Accessibility 権限

## 手順

### Part 1: メニューバー構造の確認

1. SDIT をバックグラウンドで起動する
2. window-info でウィンドウの存在を確認する（最大 15 秒ポーリング）
3. `list-menus.sh sdit` でメニューバー構造を JSON 取得する
4. 以下のメニュー構成が存在することを確認する:
   - **SDIT**: About SDIT, Settings…, Quit SDIT
   - **File**: New Window, New Tab, Close Tab
   - **Edit**: Copy, Paste, Select All
   - **View**: Toggle Sidebar, Zoom In, Zoom Out, Actual Size, Search…
   - **Session**: Next Tab, Previous Tab, Move Tab to New Window

### Part 2: メニューバーのスクリーンショット確認

5. SDIT をフォーカスした状態で `capture-region.sh 0 0 600 25` でメニューバー領域をキャプチャ（`tmp/028-menubar-default.png`）
6. スクリーンショットに「SDIT」「File」「Edit」「View」「Session」のメニュー名が表示されていることを視覚確認する

### Part 3: メニュー操作の確認

7. `click-menu.sh sdit "File"` で File メニューを開く
8. `capture-region.sh` でメニューが展開した状態をキャプチャ（`tmp/028-menubar-file-open.png`）
9. Escape でメニューを閉じる（`send-keys.sh sdit` + Escape）
10. `click-menu.sh sdit "File" "New Window"` で新ウィンドウを生成する
11. window-info で 2 つのウィンドウが存在することを確認する
12. `click-menu.sh sdit "File" "New Tab"` で新タブを追加する
13. window-info でサイドバーが出現していることを確認する（ウィンドウ幅変化等）

### Part 4: Edit メニューの確認

14. send-keys.sh で `echo MENU_TEST` + Return を入力する
15. `click-menu.sh sdit "Edit" "Select All"` で全選択する
16. `click-menu.sh sdit "Edit" "Copy"` でコピーする
17. pbpaste でクリップボードの内容を確認する

### Part 5: View メニューの確認

18. `click-menu.sh sdit "View" "Zoom In"` でフォントサイズ拡大
19. capture-window でキャプチャし、テキストが拡大されていることを視覚確認する（`tmp/028-zoom-in.png`）
20. `click-menu.sh sdit "View" "Actual Size"` でリセットする

## 期待結果
- list-menus.sh の JSON に 5 つのメニュー（SDIT, File, Edit, View, Session）が含まれる
- 各メニューに期待される項目名が存在する
- File > New Window でウィンドウが増える
- File > New Tab でタブが追加される
- Edit > Select All + Copy でクリップボードにテキストがコピーされる
- View > Zoom In でフォントサイズが変わる
- `tmp/028-menubar-default.png` にメニュー名が表示されている

## クリーンアップ
- SDIT プロセスを終了する
- `tmp/028-*.png` を削除する

## 関連
- Phase 11.1: macOS メニューバー (`docs/plans/phase11.1-macos-menubar.md`)
- `crates/sdit/src/menu.rs` — muda メニュー構築
- `crates/sdit/src/main.rs` — NSApp メニュー設定
- `crates/sdit/src/event_loop.rs` — MenuAction ディスパッチ
