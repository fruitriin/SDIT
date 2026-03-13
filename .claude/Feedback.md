# Process Feedback

開発プロセスの振り返りと改善を記録する。

## 記録方法

タスク完了時や問題発生時に、以下のいずれかのセクションに追記する。

## オーナーフィードバック

（なし）

## 問題の記録

（なし）

## 改善アクション

（なし）

## 完了済み

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
