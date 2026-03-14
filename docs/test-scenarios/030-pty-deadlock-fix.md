# 030: PTY デッドロック修正確認

## 目的
PTY リーダースレッドが Mutex ロック外で `write_tx.send()` を実行するよう修正されたことを確認する。
高頻度な PTY 応答（DA/DSR/CPR 等）が発生しても、メインスレッドが停滞しないことを検証する。

## 背景
Phase 21.4: `drain_pending_writes()` の結果を Mutex ロック内で回収し、ロック解放後に
`write_tx.send()` で送信するよう変更。Mutex 保持中に `SyncSender.send()` がブロックすると
メインスレッドが停滞する問題を修正。

## 前提条件
- `cargo build --package sdit`
- `tools/test-utils/build.sh`
- Screen Recording 権限 + Accessibility 権限

## 手順

### 基本動作確認
1. SDIT をバックグラウンドで起動する
2. window-info でウィンドウの存在を確認する（最大 15 秒ポーリング）
3. send-keys で `echo hello` を入力して Return を送信する
4. 2 秒待機する
5. capture-window でスクリーンショットを撮る（`tmp/030-basic.png`）
6. verify-text で "hello" の表示を確認する

### 高頻度 PTY 応答テスト
7. send-keys で `printf '\e[c'` を入力して Return を送信する（DA1 クエリ）
8. 1 秒待機する
9. send-keys で `printf '\e[>c'` を入力して Return を送信する（DA2 クエリ）
10. 1 秒待機する
11. capture-window でスクリーンショットを撮る（`tmp/030-da-query.png`）
12. ウィンドウが応答していること（フリーズしていないこと）を確認する

### メニュー操作との組み合わせ
13. File > New Window でウィンドウを追加する
14. 新しいウィンドウで send-keys で `echo newwin` を入力して Return を送信する
15. 2 秒待機する
16. capture-window でスクリーンショットを撮る（`tmp/030-newwin.png`）
17. verify-text で "newwin" の表示を確認する

## 期待結果
- 基本的な echo が正常に表示される
- DA1/DA2 クエリ後もウィンドウがフリーズしない
- メニュー操作後も正常に動作する
- すべての操作でプロセスがハングアップしない

## クリーンアップ
- 全 SDIT プロセスを終了する
- `tmp/030-*.png` を削除する

## 関連
- `crates/sdit/src/window.rs` — `spawn_pty_reader()` の `drain_pending_writes` 処理
- `docs/plans/phase21.4-pty-deadlock.md`
- `docs/knowhow/pty-threading-model.md`
