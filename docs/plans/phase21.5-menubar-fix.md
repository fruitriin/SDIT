# Phase 21.5: macOS ネイティブメニューバーが反映されない問題の修正

## 問題

macOS のネイティブメニューバー（画面左上）に SDIT 独自のメニュー（File/Edit/View/Session）が表示されず、macOS デフォルトのメニュー項目（About, Services, Hide 等）しか表示されない。

## 原因

`main.rs` で `menu_bar.init_for_nsapp()` を `event_loop.run_app()` の **前** に呼んでいる（L56）。

winit 0.30 の macOS 実装は `run_app()` 内で `NSApplication` のセットアップを行い、その過程で **デフォルトのメニューバーで上書き** する。そのため、事前に設定した muda メニューが消える。

```
現在の実行順序:
1. build_menu_bar() → muda メニュー構築
2. menu_bar.init_for_nsapp() → NSApp にメニュー設定 ← ここで設定するが...
3. event_loop.run_app() → winit が NSApp を初期化 → メニューバーが上書きされる ← ここで消える
4. resumed() → ウィンドウ生成
```

## 修正方針

`init_for_nsapp()` の呼び出しを `resumed()` コールバック内に移動する。winit が NSApp を初期化した **後** にメニューを設定すれば上書きされない。

```
修正後の実行順序:
1. build_menu_bar() → muda メニュー構築（main.rs、事前に構築は OK）
2. event_loop.run_app() → winit が NSApp を初期化（デフォルトメニュー設定）
3. resumed() → menu_bar.init_for_nsapp() → NSApp にメニュー設定（上書き成功）
4. ウィンドウ生成
```

### 具体的な変更

1. **`main.rs`**: `menu_bar.init_for_nsapp()` の呼び出しを削除。`menu_bar` を `SditApp` に渡す
2. **`app.rs`**: `SditApp` に `menu_bar: Option<Menu>` フィールドを追加
3. **`event_loop.rs`**: `resumed()` の冒頭で `menu_bar.init_for_nsapp()` を呼び、`Option` を `None` にして1回だけ実行されるようにする

## 変更対象

- `crates/sdit/src/main.rs` — `init_for_nsapp()` 削除、menu_bar を SditApp に渡す
- `crates/sdit/src/app.rs` — `SditApp` に menu_bar フィールド追加
- `crates/sdit/src/event_loop.rs` — `resumed()` 内で `init_for_nsapp()` 呼び出し

## テスト

- テストシナリオ `028-macos-menubar.md` で検証
  - `list-menus.sh sdit` で 5 メニュー（SDIT/File/Edit/View/Session）が存在すること
  - `capture-region.sh 0 0 600 25` でメニューバーのスクリーンショット確認
  - `click-menu.sh sdit "File" "New Window"` で実際にメニュー操作が動作すること

## セキュリティ影響

なし
