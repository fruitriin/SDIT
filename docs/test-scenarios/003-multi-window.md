# 003: マルチウィンドウ独立セッション確認

## 目的
Cmd+N で新しいウィンドウが生成され、各ウィンドウが独立した PTY セッションを持つことを確認する。

## 前提条件
- `cargo build --package sdit`
- `tools/test-utils/build.sh`
- Screen Recording 権限 + OS 再起動
- Accessibility 権限

## 手順
1. SDIT をバックグラウンドで起動する
2. window-info でウィンドウの存在を確認する（最大 15 秒ポーリング）
3. send-keys で Cmd+N を送信して新しいウィンドウを生成する
4. 1 秒待機する（ウィンドウ生成を待つ）
5. window-info で 2 つのウィンドウが存在することを確認する
6. 1 つ目のウィンドウを対象に send-keys で `echo WIN_A_SESSION` を入力し、Return を送信する
7. 2 つ目のウィンドウを対象に send-keys で `echo WIN_B_SESSION` を入力し、Return を送信する
8. 2 秒待機する（PTY 出力 → 描画の伝搬を待つ）
9. capture-window で 1 つ目のスクリーンショットを撮る（`tmp/003-win-a.png`）
10. capture-window で 2 つ目のスクリーンショットを撮る（`tmp/003-win-b.png`）

## 期待結果
- ウィンドウが 2 つ存在する（window-info が 2 ウィンドウを報告）
- `tmp/003-win-a.png` のファイルサイズが 10 KiB 以上（空白でない）
- `tmp/003-win-b.png` のファイルサイズが 10 KiB 以上（空白でない）
- （将来）AI 視覚分析で 1 つ目のスクリーンショットに "WIN_A_SESSION" が描画されていることを確認
- （将来）AI 視覚分析で 2 つ目のスクリーンショットに "WIN_B_SESSION" が描画されており、"WIN_A_SESSION" は含まれていないことを確認（セッション独立性）

## クリーンアップ
- SDIT プロセスを終了する（全ウィンドウを閉じる）
- `tmp/003-win-a.png` を削除する
- `tmp/003-win-b.png` を削除する

## 関連
- Phase 3: SDI マルチウィンドウ
- `crates/sdit/src/app.rs` の新規ウィンドウ生成処理
- `crates/sdit/src/window_ops.rs` のウィンドウ操作ユーティリティ
