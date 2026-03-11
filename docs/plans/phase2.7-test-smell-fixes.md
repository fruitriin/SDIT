# Phase 2.7 — テストスメル修正

savanna-smell-detector で検出されたテストスメルを修正し、テストチェーンに組み込む。

## 出典

`.claude/Feedback.md` より（Phase 2.5 で savanna-smell-detector 導入時に記録）

## タスク

- [x] Missing Assertion: `test_pty_spawn_shell`, `test_pty_resize` にアサーション追加
  - `test_pty_spawn_shell`: spawn 直後に `try_wait()` で子プロセス生存を確認
  - `test_pty_resize`: resize 後に `try_wait()` で子プロセス生存を確認
- [x] Conditional Test Logic: **対応不要**
  - `if !is_tty()` スキップは CI/サンドボックス環境で PTY ioctl が ENOTTY になるため構造的に必要
  - `docs/knowhow/macos26-pty-compat.md` に背景を記録済み
- [x] Sleepy Test: **対応不要（現時点で最適解）**
  - PTY read はブロッキング IO のため `deadline + sleep(10ms)` ポーリングが必要
  - `std::process::Child::wait_timeout()` は nightly only（stable では使用不可）
  - 10ms sleep は CPU 負荷も許容範囲内
- [ ] テストチェーンに `savanna-smell-detector --min-severity 4 --fail-on-smell` を組み込む

## 対象ファイル

- `crates/sdit-core/src/pty/` 配下のテスト
- テストチェーン（cargo test 後に実行）

## 完了条件

- [x] Missing Assertion の修正完了
- [x] smell-detector のテストチェーン組み込み完了（`scripts/check.sh`）
- [x] `cargo test` 全通過
- [x] `savanna-smell-detector --min-severity 4 --fail-on-smell` 通過
