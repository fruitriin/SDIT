# 035: ウィンドウフォーカス取得時のマウスクリック抑制確認

## 目的
`[mouse] swallow_mouse_click_on_focus = true` 設定時、フォーカスされていないウィンドウをクリックしたとき、フォーカスのみ取得してクリック入力がターミナルに渡らないことを確認する。

## 前提条件
- `cargo build --package sdit`
- `tools/test-utils/build.sh`
- Screen Recording 権限 + Accessibility 権限

## 手順

### A: swallow_mouse_click_on_focus = true

1. 設定ファイルに以下を追加する:
   ```toml
   [mouse]
   swallow_mouse_click_on_focus = true
   ```
2. SDIT をバックグラウンドで起動する
3. window-info でウィンドウの存在を確認する（最大 15 秒ポーリング）
4. send-keys で `echo BEFORE_CLICK` + Return を入力する
5. 2 秒待機する
6. 別のアプリケーション（例: Finder）をアクティブにして SDIT からフォーカスを外す
7. 1 秒待機する
8. SDIT ウィンドウをクリックしてフォーカスを戻す
9. 1 秒待機する
10. capture-window でスクリーンショットを撮る（`tmp/035-swallow-true.png`）
11. SDIT を終了する

### B: swallow_mouse_click_on_focus = false（デフォルト）

1. 設定ファイルで `swallow_mouse_click_on_focus = false` にする（またはキーを削除する）
2. SDIT をバックグラウンドで起動する
3. window-info でウィンドウの存在を確認する
4. send-keys で `echo BEFORE_CLICK` + Return を入力する
5. 2 秒待機する
6. 別のアプリケーションをアクティブにして SDIT からフォーカスを外す
7. 1 秒待機する
8. SDIT ウィンドウをクリックしてフォーカスを戻す
9. 1 秒待機する
10. capture-window でスクリーンショットを撮る（`tmp/035-swallow-false.png`）
11. SDIT を終了する

### C: 設定未指定（デフォルト false と同等）

1. 設定ファイルから `swallow_mouse_click_on_focus` を削除する
2. SDIT をバックグラウンドで起動する
3. window-info でウィンドウの存在を確認する
4. SDIT を終了する

## 期待結果

### A（true）
- クリックで SDIT ウィンドウにフォーカスが戻る
- クリック入力はターミナルに渡らない（カーソル位置が変わらない、選択が発生しない）
- フォーカス取得後の2回目以降のクリックは通常どおりターミナルに届く

### B（false）
- クリックで SDIT ウィンドウにフォーカスが戻る
- クリック入力はターミナルにも渡される（標準 macOS 挙動）

### C（デフォルト）
- B と同じ挙動（デフォルト false）
- SDIT が正常に起動・終了する

## ユニットテスト確認

```bash
# 設定のデシリアライズ確認
cargo test swallow -- --nocapture
# デフォルト値が false であることを config テストで確認
cargo test config -- --nocapture
```

## クリーンアップ
- SDIT プロセスを終了する
- `tmp/035-*.png` を削除する
- テスト用設定ファイルを元に戻す

## 関連
- Phase 25.4: `docs/plans/phase25.4-swallow-mouse-click-on-focus.md`
- `crates/sdit/src/window_ops.rs` — `handle_focused` メソッド
- `crates/sdit/src/app.rs` — `just_focused` フィールド
- `crates/sdit-core/src/config/mod.rs` — `[mouse] swallow_mouse_click_on_focus` 設定
- WezTerm 参照: `swallow_mouse_click_on_window_focus`
