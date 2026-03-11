# Phase 2.7 — テストスメル修正

savanna-smell-detector で検出されたテストスメルを修正し、テストチェーンに組み込む。

## 出典

`.claude/Feedback.md` より（Phase 2.5 で savanna-smell-detector 導入時に記録）

## タスク

- [ ] Missing Assertion: `test_pty_spawn_shell`, `test_pty_resize` にアサーション追加
- [ ] Conditional Test Logic: PTY テストの `if !is_tty()` スキップパターン（CI環境対応、構造的に必要）→ 対応不要の判断を記録
- [ ] Sleepy Test: PTY read のタイムアウト待機（ブロッキング IO に起因）→ 代替手段を検討、対応可能なら対応
- [ ] テストチェーンに `savanna-smell-detector --min-severity 4 --fail-on-smell` を組み込む

## 対象ファイル

- `crates/sdit-core/src/pty/` 配下のテスト
- テストチェーン（cargo test 後に実行）

## 完了条件

- Missing Assertion の修正完了
- smell-detector のテストチェーン組み込み完了
- `cargo test` 全通過
