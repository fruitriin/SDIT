# Phase 3.1 — セキュリティ修正（Phase 3 セキュリティレビュー由来）

Phase 3 セキュリティレビューで検出された Medium 2件の修正。

## 出典

Phase 3 セキュリティレビュー結果（M-1, M-2）

## M-1 | Medium | PTY リサイズ時に SIGWINCH が送信されない

**場所:** `main.rs` `handle_resize()`

`terminal.resize()` はグリッドを更新するが、`Pty::resize()` が呼ばれないため子プロセスに SIGWINCH が送信されない。Phase 2 からの既知の問題（knowhow `pty-threading-model.md` L81 に記載）だが、Phase 3 で複数ウィンドウになり影響が拡大。

**影響:** vim/less 等の fullscreen TUI アプリで表示崩れ。メモリ安全性には影響しない。

**修正案:**
- Session に resize コマンドチャネル（`mpsc::Sender<PtyCommand>`）を追加
- Reader スレッドが読み取り合間に `try_recv()` で resize コマンドを受信
- `handle_resize()` で `session.send_resize(new_size)` を呼ぶ
- あるいは、PTY master fd をもう1つクローンして Session に保持し、`rustix::termios::tcsetwinsize()` で直接リサイズ

## M-2 | Medium | Session 削除時に PTY スレッドが join されない（リソースリーク）

**場所:** `session.rs` `PtyIo`, `main.rs` `close_window()`

Session が drop されると Reader/Writer スレッドは detach される。Writer は write_tx の drop で終了するが、Reader は pty.read() でブロック中の場合に終了しない。

**影響:** 大量のウィンドウ開閉でスレッド・PTY fd・子プロセスが蓄積。通常使用では影響限定的。

**修正案:**
- Session に `Drop` impl を追加
- 子プロセスの PID を Session に保存し、Drop 時に SIGTERM を送信
- Reader スレッドに shutdown フラグ（`Arc<AtomicBool>`）を持たせる
- non-blocking PTY read への変更を検討（polling crate 等）

## タスク

- [ ] M-1: PTY リサイズチャネルの実装
- [ ] M-2: Session Drop + スレッド join の実装
- [ ] テスト追加
- [ ] `cargo test` + `scripts/check.sh` 全通過

## 完了条件

- [ ] ウィンドウリサイズ時に子プロセスが正しいサイズを認識する
- [ ] Session drop 時にスレッドが適切に終了する
- [ ] リグレッションテスト通過
