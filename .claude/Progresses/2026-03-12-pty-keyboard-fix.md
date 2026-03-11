# 進捗表

## 運用ルール

### タスク開始時
1. .claude/Feedback.md を読み、前回の改善アクションで未対応のものがあれば考慮する
2. 以下の手順で Markdown チェックリストを作成する
   1. 1ショットで作業できる範囲にサブタスクを分割する
   2. 並行作業できる粒度でさらに分割する
   3. 各サブタスクにテスト作成・統合テスト・Lint・ビルドが必要か検討し、必要なら追加する
   4. 必要に応じて 2.1〜2.3 を再帰的に適用する

### 作業中
3. サブタスク着手時に `- [x]` でチェックしていく。並列可能なタスクはコンテナオーケストレーションを利用する

### タスク完了時
4. コード変更がある場合、Lint・ビルドを通す（`cargo fmt --check && cargo clippy --all-targets && cargo test`）
5. コード変更がある場合、セキュリティレビューサブエージェントを起動する（CLAUDE.md「セキュリティレビュー方針」参照）
   - 変更差分に対して脆弱性の有無を検査し、修正案を提示させる（実装はサブエージェントにさせない）
   - 発見された脆弱性を Progress のチェックリストにサブタスクとして追加する
   - 追加したサブタスクを修正・完了させる（このフェーズ内で解決する）
   - 修正結果を該当フェーズの Plan ファイルに記録する
   - バイナリが動作する段階（Phase 2以降）では、ペネトレーションテストの必要性も検討する
6. バイナリが動作する段階（Phase 2以降）では、リグレッションテストサブエージェントを起動する（CLAUDE.md「リグレッションテスト方針」参照）
   - 変更内容に基づいて退行リスクのあるテストケースを計画・提案させる
   - 提案されたテストを実装・実行し、既存機能の退行がないことを確認する
7. 投入されたタスクのPlanに実装完了状況を反映する
8. .claude/Feedback.md にPlan, TODO, Progress推進エンジンの問題の記録・改善アクションを追記する。反映済みの項目は削除する
9. .claude/Feedback.md にプロジェクト進行上の問題の記録・改善アクションを追記する。反映済みの項目は削除する
10. `.claude/Progresses/YYYY-MM-DD-プラン名.md` にリネームして移動し、`.claude/templates/ProgressTemplate.md` から新規の Progress.md を作成する
11. 実装の知見で継続して効果が見込めるもの、再調査が必要なものを docs/knowhow に.mdで作成する。

12. コミットする

---

## タスク: PTY キー入力デッドロック修正

- [x] 根本原因調査: `spawn_pty_reader` が1スレッドで read/write を処理 → ブロッキング read が write をブロック
- [x] `sdit-core`: `Pty::try_clone_writer()` メソッド追加（`AsFd::try_clone_to_owned()` で master fd クローン）
- [x] `main.rs`: `spawn_pty_reader` (read のみ) と `spawn_pty_writer` (write のみ) に分離
- [x] ビルド・テスト・clippy 全パス
- [x] セキュリティレビュー実施（Critical/High: 0件）
  - [x] SV-2 (Medium): `try_send` サイレント破棄 → `log::warn!` 追加で修正
  - [x] SV-4 (Low): ライタースレッド異常終了 → `ChildExit` 通知追加で修正
  - SV-1 (Medium/既存): 入力長制限 — 既存問題、Phase 2 Plan に記録
  - SV-3, SV-5, SV-6 (Low/既存): Plan に記録
- [x] Plan ファイル更新
- [x] Feedback.md 更新（gui入力確認項目を対応済みに更新）
- [x] knowhow 記録
