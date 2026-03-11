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

## タスク: Phase 2 — 最初のSDIウィンドウ

### Step 1: winit + wgpu 空ウィンドウ表示 ✓ 完了（2026-03-11）
- [x] sdit-render: `GpuContext` 構造体（wgpu Device/Queue/Surface 初期化）
- [x] sdit binary: `SditApp` 構造体 + `ApplicationHandler` trait 実装
- [x] sdit binary: カスタムイベント型 `SditEvent` 定義
- [x] winit EventLoop 起動 → 空ウィンドウ表示（背景色クリアのみ）
- [x] ビルド確認（cargo fmt/clippy/test 全通過、警告ゼロ）

### Step 2: フォント読み込み + テクスチャアトラス
- [ ] sdit-render: `Atlas` 構造体（ビンパッキング、wgpu テクスチャ管理）
- [ ] sdit-render: cosmic-text `FontSystem` + `SwashCache` でグリフラスタライズ
- [ ] sdit-render: `FontContext` 構造体（セルメトリクス計算・グリフキャッシュ）
- [ ] テスト: アトラスへのグリフ配置・メトリクス計算

### Step 3: グリッドレンダリングパイプライン
- [ ] sdit-render: WGSL シェーダー（背景色 + テキスト描画）
- [ ] sdit-render: `RenderPipeline` 構造体（wgpu パイプライン・バインドグループ）
- [ ] sdit-render: Grid → `RenderableCell` 変換（背景色配列 + 前景テキスト配列）
- [ ] sdit-render: `draw()` 関数（セルバッファ → GPU バッファ同期 → 描画）
- [ ] 静的テキスト描画の動作確認

### Step 4: PTY スレッド接続 + Terminal 状態共有
- [ ] sdit binary: `Arc<Mutex<Terminal>>` で Terminal 状態共有
- [ ] sdit binary: PTY reader スレッド（polling/read → VTE parse → Terminal 更新）
- [ ] sdit binary: PTY → `EventLoopProxy::send_event()` で再描画要求
- [ ] sdit binary: ウィンドウリサイズ → PTY resize + Grid resize
- [ ] シェル出力がウィンドウに表示されることを確認

### Step 5: キー入力 → PTY 送信
- [ ] sdit binary: winit `KeyEvent` → バイト列変換（基本キー + 修飾キー）
- [ ] sdit binary: Main → PTY writer チャネル（`std::sync::mpsc`）
- [ ] 対話的なシェル操作の動作確認（ls, cd, echo 等）

### 完了処理
- [ ] `cargo fmt --check && cargo clippy --all-targets && cargo test`
- [ ] セキュリティレビューサブエージェント起動・指摘対応
- [ ] リグレッションテスト計画・実施
- [ ] Plan ファイル・TODO・Feedback 更新
- [ ] knowhow 記録・コミット
