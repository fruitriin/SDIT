# 031: タブのドラッグ切り出し（Chrome-like UX）

## 目的
サイドバーのタブをドラッグしてウィンドウ外に出すと、セッションが独立したウィンドウに切り出されることを確認する。GUI テストではドラッグ操作のシミュレーションが困難なため、メニュー操作（Session > Move Tab to New Window）による DetachSession の代替確認を行う。

## 前提条件
- `cargo build --package sdit`
- `tools/test-utils/build.sh`
- Screen Recording 権限 + OS 再起動
- Accessibility 権限

## 手順

### Part A: メニュー操作による DetachSession 確認（代替テスト）
1. SDIT をバックグラウンドで起動する
2. window-info でウィンドウの存在を確認する（最大 15 秒ポーリング）
3. capture-window でスクリーンショットを撮る（`tmp/031-initial.png`）：1ウィンドウ・サイドバーなし
4. send-keys で Cmd+T を送信する（2つ目のセッションを追加）
5. 1 秒待機する（セッション生成 → サイドバー出現を待つ）
6. send-keys で `echo TAB_DETACH_TEST` を入力し、Return を送信する
7. 1 秒待機する（PTY 出力 → 描画の伝搬を待つ）
8. capture-window でスクリーンショットを撮る（`tmp/031-two-tabs.png`）：2タブ・サイドバー表示
9. Session メニューから "Move Tab to New Window" を選択する（Cmd+Shift+N でも可）
10. 2 秒待機する（ウィンドウ生成 → 描画の伝搬を待つ）
11. window-info でウィンドウ一覧を取得し、2 ウィンドウが存在することを確認する
12. 各ウィンドウの capture-window でスクリーンショットを撮る（`tmp/031-win-a.png`, `tmp/031-win-b.png`）

### Part B: 退行確認
13. 各ウィンドウがサイドバーなし（1セッション状態）であることを確認する
14. 各ウィンドウが独立した PTY セッションで動作していることを確認する（一方に send-keys で入力し、他方に表示されないことを確認）

## 期待結果
- Step 5 後: サイドバーが出現し 2 タブが表示されている
- Step 11: window-info が 2 ウィンドウを報告する
- `tmp/031-win-a.png`: 1 セッション状態（サイドバーなし）
- `tmp/031-win-b.png`: 1 セッション状態（サイドバーなし）
- 切り出されたセッションの PTY が維持されている（"TAB_DETACH_TEST" がいずれかのウィンドウに表示されている）
- 各スクリーンショットのファイルサイズが 10 KiB 以上（空白でない）

## 注意事項
- ドラッグ操作のテストは GUI 自動テストでの再現が困難。CursorLeft イベントベースの実装は手動テストで確認する
- メニュー操作（Cmd+Shift+N）による DetachSession は既存の 007-session-detach と同等だが、Phase 22.1 の実装変更による退行がないことを確認する目的

## クリーンアップ
- SDIT プロセスをすべて終了する
- `tmp/031-*.png` を削除する

## 関連
- Phase 22.1 計画: `docs/plans/phase22.1-tab-drag-detach.md`
- 007-session-detach: 既存の DetachSession テストシナリオ
- `crates/sdit/src/event_loop.rs` — CursorLeft でのドラッグ切り出し処理
- `crates/sdit/src/window_ops.rs` — `detach_session_to_new_window()` 位置指定パラメータ
