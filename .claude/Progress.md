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

#### Stage 2: 品質検証チーム（並列実行）

5. `quality-gate` チーム（TeamCreate）を作成し、以下の2エージェントを **並列** で起動する:

   **[security-review エージェント]** — セキュリティレビュー
   - 変更差分（`git diff`）に対して脆弱性の有無を検査し、修正案を提示する（実装はしない）
   - 発見した脆弱性を重要度付き（Critical/High/Medium/Low/Info）で報告する
   - バイナリが動作する段階では、ペネトレーションテストの必要性も検討する

   **[integration-test エージェント]** — 統合テスト・リグレッションテスト
   - 変更内容に基づいて退行リスクのあるテストケースを計画・提案する
   - `docs/test-scenarios/` の該当シナリオがあれば `/gui-test` で実行する
   - 提案されたテストを実装・実行し、既存機能の退行がないことを確認する

6. チームの報告を集約し、フィードバックに基づいてサブタスクを追加する:
   - **Critical/High**: 必ずこのフェーズ内で修正する（先送り禁止）
   - **Medium**: 原則修正。先送りする場合は `phaseX.Y-security-fixes.md` として独立計画を起こす
   - **Low/Info**: Plan に記録し、必要に応じて独立計画で対応
   - **テスト提案**: 実装・実行して `cargo test` スイートに追加する
   - 修正・テスト追加後、Stage 1 を再実行して通過を確認する
   - 修正結果を該当フェーズの Plan ファイルに記録する
   - セキュリティ修正やテストで得た知見を `/knowhow` で記録する
   - **セキュリティ修正が全て完了するまでフェーズの完了コミットを行わない**

#### 完了処理

7. 投入されたタスクのPlanに実装完了状況を反映する
8. .claude/Feedback.md にPlan, TODO, Progress推進エンジンの問題の記録・改善アクションを追記する。反映済みの項目は削除する
9. .claude/Feedback.md にプロジェクト進行上の問題の記録・改善アクションを追記する。反映済みの項目は削除する
10. `.claude/Progresses/YYYY-MM-DD-プラン名.md` にリネームして移動し、`.claude/templates/ProgressTemplate.md` から新規の Progress.md を作成する
11. 実装の知見で継続して効果が見込めるもの、再調査が必要なものを docs/knowhow に.mdで作成する

12. コミットする

---

## タスク: Phase 7 — IME入力サポート（完了）

### 実装
- [x] Step 1: IME 有効化 — `window_ops.rs` で `set_ime_allowed(true)` を追加
- [x] Step 2: IME Commit 処理 — `event_loop.rs` で `Ime::Commit` → PTY 送信
- [x] Step 3: PreeditState 構造体追加 — `app.rs` に preedit 状態管理を追加
- [x] Step 4: IME Preedit イベント処理 — `event_loop.rs` で `Ime::Preedit` → 状態更新
- [x] Step 5: IME カーソル位置通知 + プリエディット描画 — `render.rs`

### 品質ゲート
- [x] Stage 1: `cargo fmt --check && cargo clippy --all-targets && cargo test` — 全通過（警告0）

### 完了処理
- [x] Plan 更新 (`docs/plans/phase7-ime.md`)
- [x] TODO 更新 (Phase 7 → done)
- [x] knowhow 記録 (`docs/knowhow/ime-input-support.md`)
