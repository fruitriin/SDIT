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

- [x] M-1: PTY リサイズ — `Pty::try_clone_resize_fd()` で `OwnedFd` を Session に保持、`Session::resize_pty()` で `rustix::termios::tcsetwinsize()` 呼び出し
- [x] M-2: Session Drop — SIGHUP → Writer join(200ms) → Reader join(300ms) + SIGKILL エスカレーション
- [x] main.rs `handle_resize()` から `session.resize_pty()` 呼び出し
- [x] `cargo test` + `scripts/check.sh` 全通過
- [x] セキュリティレビュー実施

## セキュリティレビュー結果（Phase 3.1 実装に対するレビュー）

| ID | 重要度 | 内容 | 対処 |
|---|---|---|---|
| S-1 | **High** | `unwrap_or(1)` で PID 変換失敗時に PID 1 (init/launchd) へ SIGHUP 送信リスク | **修正済み**: `to_rustix_pid()` ヘルパーで `i32::try_from().ok()` + `Pid::from_raw()` → None で安全にスキップ。加えて `child_exited: Arc<AtomicBool>` フラグで PID 再利用による誤送信も防止 |
| S-2 | Low | `wait_and_join()` のビジーウェイト（10ms sleep ループ） | 記録のみ。`condvar` や `thread::park` への置き換えは将来検討 |
| S-3 | **Medium** | Writer/Reader の共有デッドライン — Writer が遅延すると Reader のタイムアウトが短縮 | **修正済み**: Writer に 200ms、Reader に 300ms の独立デッドラインを割り当て |
| S-4 | Low | `SyncSender` の `Full` エラー時にデータがサイレントドロップ | 記録のみ。Phase 2 からの既知問題。バックプレッシャー設計は Phase 5 で検討 |

## 完了条件

- [x] ウィンドウリサイズ時に子プロセスが正しいサイズを認識する（`tcsetwinsize` 経由で SIGWINCH 送信）
- [x] Session drop 時にスレッドが適切に終了する（SIGHUP → timeout → SIGKILL）
- [x] リグレッションテスト通過（`scripts/check.sh` 全通過）
- [x] セキュリティレビュー — Critical/High 修正済み、Medium 修正済み、Low 記録済み
