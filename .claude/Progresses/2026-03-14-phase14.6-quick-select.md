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
4. 実装フェーズの最終サブタスク完了時、実装で得た知見を `/knowhow` で記録する（既存 knowhow の更新も含む）

### エージェント起動時の共通ルール
- エージェントチーム（TeamCreate）やサブエージェント（Agent）を作成するとき、各エージェントへのプロンプトに **最初に `/knowhow-index` を実行する** よう指示を含めること
- これにより各エージェントがプロジェクトの知見ベースを把握した状態で作業を開始できる

### タスク完了時 — 2段階品質ゲート

#### Stage 1: ビルド検証（ゲートキーパー）

4. `cargo fmt --check && cargo clippy --all-targets && cargo test` を実行する
   - **失敗した場合 → 実装に差し戻す**。プランエージェントで原因分析 → 実装エージェントで修正 → Stage 1 を再実行
   - Stage 1 を通過するまで Stage 2 に進まない
   - **リグレッションテスト**: `docs/test-scenarios/INDEX.md` で最終実行日が古いシナリオ（老番）を確認し、変更に関連しうるものがあれば Stage 2 の integration-test で優先的に実行する

#### Stage 2: 品質検証チーム（並列実行）

5. `quality-gate` チーム（TeamCreate）を作成し、以下の2エージェントを **並列** で起動する:

   **[security-review エージェント]** — セキュリティレビュー
   - 変更差分（`git diff`）に対して脆弱性の有無を検査し、修正案を提示する（実装はしない）
   - 発見した脆弱性を重要度付き（Critical/High/Medium/Low/Info）で報告する
   - バイナリが動作する段階では、ペネトレーションテストの必要性も検討する

   **[integration-test エージェント]** — 統合テスト・シナリオ管理
   - **重要: `subagent_type` は未指定（general-purpose）を使うこと**（Bash/Edit/Write が必要。Explore は読み取り専用で GUI テストを実行できない）
   - **テストシナリオの追加・ブラッシュアップ**: 今回の Plan に対応するテストシナリオを `docs/test-scenarios/` に新規作成、または既存シナリオを更新する
   - **選択的テスト実施**: 全シナリオではなく、変更に関連するシナリオを選んで `/gui-test` で実行する（SDIT バイナリを起動し、スクリーンショットを撮って検証する）
   - **リグレッション優先**: Stage 1 で特定された老番シナリオがあれば、新規シナリオより先に実行する
   - **適応的実行モード**: 画面操作が安定している場合は複数操作をまとめて投機的に実行し、不安定（予期しない状態）になったらステップ実行に切り替える。これによりトークン消費を抑えつつ網羅性を確保する
   - **シナリオインデックス更新**: `docs/test-scenarios/INDEX.md` を更新する（最終実行日時を記録）
   - **退行確認**: 実行したシナリオで既存機能の退行がないことを確認する
   - **補助ツール提案**: テスト実施中にツール（キャプチャ、比較、分析等）の不足を感じたら、改善アクションとして Feedback.md に記録する
   - **Teardown**（メインプロセスから打ち切り指示を受けたとき）: 実行結果レポートを返し、得た知見を `/knowhow` で記録する

   > `docs/test-scenarios/INDEX.md` の形式は `/knowhow-index` と類似:
   > | シナリオ | 要約 | 最終実行 | 結果 |
   > シナリオ追加・更新時は必ずインデックスも更新する。

6. **Stage 2 の制御フロー**:
   - security-review と integration-test は **並列** で開始する
   - セキュリティレビューの指摘 → **実装エージェント（sonnet）に修正を移譲する**（メインプロセスが直接修正するのではなく、修正案を実装エージェントに渡して修正させる）
   - **セキュリティ修正が全て完了したら integration-test に Teardown を指示する**
   - integration-test は Teardown 指示を受けたら、実行中のテストを区切りよく終え、結果レポート + knowhow 書き出しを行って終了する

7. フィードバックの集約:
   - **Critical/High**: 必ずこのフェーズ内で修正する（先送り禁止）
   - **Medium**: 原則修正。先送りする場合は `phaseX.Y-security-fixes.md` として独立計画を起こす
   - **Low/Info**: Plan に記録し、必要に応じて独立計画で対応
   - **テスト提案**: 実装エージェントに移譲し、実装・実行して `cargo test` スイートに追加する
   - **バグ分離**: 統合テストで発見されたバグが現在のプランと関心事が異なる場合は、修正せずに新しいプラン（`docs/plans/phaseX.Y-*.md`）を書き起こし、TODO.md に追加するのみで現在のプランを完了させる
   - 修正・テスト追加後、Stage 1 を再実行して通過を確認する
   - 修正結果を該当フェーズの Plan ファイルに記録する
   - **セキュリティ修正が全て完了するまでフェーズの完了コミットを行わない**

#### 完了処理

8. 投入されたタスクのPlanに実装完了状況を反映する
9. .claude/Feedback.md にPlan, TODO, Progress推進エンジンの問題の記録・改善アクションを追記する。反映済みの項目は削除する
10. .claude/Feedback.md にプロジェクト進行上の問題の記録・改善アクションを追記する。反映済みの項目は削除する
11. `.claude/Progresses/YYYY-MM-DD-プラン名.md` にリネームして移動し、`.claude/templates/ProgressTemplate.md` から新規の Progress.md を作成する
12. Progress 推進エンジン自体に関するフィードバック・ノウハウがあれば、テンプレート（`.claude/templates/ProgressTemplate.md`）の改善案を Feedback.md に記録する

13. コミットする

---

## タスク: Phase 14.6 — Quick Select

### 実装
- [ ] url_detector.rs 拡張: ファイルパス・git ハッシュ・数値パターン追加 + detect_patterns_in_line()
- [ ] QuickSelectState 構造体 + ヒントラベル生成（app.rs）
- [ ] QuickSelectConfig 追加（config/mod.rs）
- [ ] Action::QuickSelect キーバインド（keybinds.rs）
- [ ] event_loop.rs: QuickSelect モード開始 + ヒントキー入力処理
- [ ] render.rs: オーバーレイ描画（ハイライト + ヒントラベル）
- [ ] テスト（パターンマッチ + ヒントラベル割り当て）

### 品質ゲート
- [ ] Stage 1: `cargo fmt --check && cargo clippy --all-targets && cargo test`
- [ ] Stage 2: security-review + integration-test
