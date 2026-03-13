# Process Feedback

開発プロセスの振り返りと改善を記録する。

## 記録方法

タスク完了時や問題発生時に、以下のいずれかのセクションに追記する。

## オーナーフィードバック

## 問題の記録

worktree 内の統合テストエージェントがフックエラーでブロックされた
CLAUDE.md に `cp -r .claude <worktree>/.claude` ルールがあるが、自動的に実行されていないケースがある。worktree 作成後の .claude コピーが漏れた場合、全ツール（Bash/Edit/Write）が使えなくなる。さらに、残存 worktree が本体のフック実行にも影響した（hook の相対パスが worktree ディレクトリから解決されるため）。

古い worktree の残存がメインプロセスのフック実行をブロックした
`.claude/worktrees/` に不要な worktree ディレクトリが残ると、PreToolUse フックが worktree 側の .claude/hooks を参照しようとしてエラーになる。チーム完了後は worktree を確実に削除する必要がある。

## 改善アクション

依存パッケージ提案ルール
実装計画時に、追加の依存パッケージで実装が容易になる場合は Plan の段階で依存クレート候補を明記し、Feedback に提出すること。オーナーが判断できるよう、クレート名・用途・代替手段を記載する。

## 完了済み

- セキュリティ/統合テストのフィードバック修正を実装エージェントに移譲 → Progress テンプレート Stage 2 制御フローに反映
- 統合テストをリグレッションとして最初に実行 → Progress テンプレート Stage 1 にリグレッション優先ルール追加
- 統合テストの適応的実行モード（安定→投機的、不安定→ステップ） → Progress テンプレート integration-test に反映
- 関心事の異なるバグは新プランに分離 → Progress テンプレート フィードバック集約にバグ分離ルール追加
- CJK テスト右端輝度分析 → `docs/knowhow/gui-test-cjk-validation.md` に記録
- E2E テスト補助ツール提案義務 → Progress テンプレート integration-test に反映
- 和文テキスト対照群 → `docs/knowhow/gui-test-cjk-validation.md` に検討結果を記録
- 計画ファイルの関心事別分離 → CLAUDE.md ブートシーケンスの Plan 作成ルールに追記

- integration-test エージェントが `subagent_type: Explore`（読み取り専用）で起動されていたため GUI テスト（SDIT 起動 + スクリーンショット）を実行できていなかった → テンプレート・CLAUDE.md を修正し `general-purpose` を使うよう明記
- worktree 起動時に `.claude` ディレクトリ（hooks 等の .gitignore 対象ファイル含む）が複製されず、フックエラーでエージェントがブロックされていた → CLAUDE.md に `cp -r .claude <worktree>/.claude` ルールを追記

- ノウハウの読み込み順のブラッシュアップ → CLAUDE.md ブートシーケンスを knowhow サブエージェントフィルタリング方式に変更
- 統合テストのログチェック → `smoke_headless.rs` と `smoke_gui.rs` に `RUST_LOG=info` + 期待ログメッセージの存在確認を追加
- Phase 2 セキュリティ Low は各 Plan に記録済み（独立計画不要）
- Phase 2.5 セキュリティ Low L-1〜L-4 は `docs/plans/phase2.5-integration-testing.md` に記録済み
- Phase 2.6 セキュリティ Low L-1〜L-3 / Info I-1〜I-3 は `docs/plans/phase2.6-security-fixes.md` に記録済み
- Phase 4 セキュリティ Medium M-1〜M-3 修正済み、Low/Info は `docs/plans/phase4-session-sidebar.md` に記録済み
- Phase 5 オーナーフィードバック（日本語フォント・カラーコントラスト・統合テスト）→ Phase 5.2/5.3/5.5 で対応済み
- Phase 5 セキュリティ Medium M-1〜M-3 修正済み、Low/Info は `docs/plans/phase5-config-polish.md` に記録済み
