# 004: ウィンドウ独立クローズ確認

## 目的
1 つのウィンドウを閉じても他のウィンドウとそのセッションが影響を受けないことを確認する。

## 前提条件
- `cargo build --package sdit`
- `tools/test-utils/build.sh`
- Screen Recording 権限 + OS 再起動
- Accessibility 権限

## 手順
1. SDIT をバックグラウンドで起動する
2. window-info でウィンドウの存在を確認する（最大 15 秒ポーリング）
3. send-keys で Cmd+N を送信して 2 つ目のウィンドウを生成する
4. 1 秒待機する（ウィンドウ生成を待つ）
5. 1 つ目のウィンドウを対象に send-keys で `echo WIN_A` を入力し、Return を送信する
6. 2 つ目のウィンドウを対象に send-keys で `echo WIN_B` を入力し、Return を送信する
7. 2 秒待機する（PTY 出力 → 描画の伝搬を待つ）
8. 1 つ目のウィンドウを対象に send-keys で Cmd+W を送信してウィンドウを閉じる
9. 1 秒待機する（ウィンドウ破棄処理を待つ）
10. window-info でウィンドウが 1 つのみ残っていることを確認する
11. capture-window で残ったウィンドウのスクリーンショットを撮る（`tmp/004-remaining.png`）

## 期待結果
- クローズ後にウィンドウが 1 つのみ存在する（window-info が 1 ウィンドウを報告）
- `tmp/004-remaining.png` のファイルサイズが 10 KiB 以上（空白でない）
- （将来）AI 視覚分析で残ったウィンドウのスクリーンショットに "WIN_B" が描画されていることを確認
- （将来）AI 視覚分析で残ったウィンドウのスクリーンショットに "WIN_A" は含まれていないことを確認（セッション独立性）
- アプリケーションがクラッシュしていない（SDIT プロセスが生存している）

## クリーンアップ
- SDIT プロセスを終了する
- `tmp/004-remaining.png` を削除する

## 関連
- Phase 3: SDI マルチウィンドウ
- `crates/sdit/src/event_loop.rs` のウィンドウクローズイベント処理
- `crates/sdit/src/app.rs` のウィンドウ管理・セッション破棄処理
