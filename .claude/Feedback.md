# Process Feedback

開発プロセスの振り返りと改善を記録する。

## 記録方法

タスク完了時や問題発生時に、以下のいずれかのセクションに追記する。

## 問題の記録

- ノウハウの読み込み順のブラッシュアップしたい。下記のプロセスの妥当性を検討する（by オーナー）
  - Plan読み込み→Planをノウハウサブエージェントの渡す→ノウハウサブエージェントはノウハウを全部読み込み、プランに必要なノウハウ、有用なノウハウのパスと内容をメインコンテキストに返すようにする
- 統合テストでターミナルに出力しているログをチェックするシナリオかステップを追加したい (by オーナー)

- savanna-smell-detector 導入済み（`--min-severity 3` で 17件検出）
  - Conditional Test Logic: PTY テストの `if !is_tty()` スキップパターン（CI環境対応、構造的に必要）
  - Sleepy Test: PTY read のタイムアウト待機（ブロッキング IO に起因、代替手段要検討）
  - Missing Assertion: `test_pty_spawn_shell`, `test_pty_resize` にアサーション追加が必要
  - **次のアクション**: Phase 3 開始時に修正可能なものを対応し、テストチェーンに `--min-severity 4 --fail-on-smell` を組み込む

## 改善アクション

- Phase 2 セキュリティ Low は各 Plan に記録済み（独立計画不要）
- Phase 2.5 セキュリティ Low L-1〜L-4 は `docs/plans/phase2.5-integration-testing.md` に記録済み
- Phase 2.6 セキュリティ Low L-1〜L-3 / Info I-1〜I-3 は `docs/plans/phase2.6-security-fixes.md` に記録済み
