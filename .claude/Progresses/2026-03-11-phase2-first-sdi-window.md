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

### Step 2: フォント読み込み + テクスチャアトラス ✓ 完了（2026-03-11）
- [x] sdit-render: `Atlas` 構造体（シェルフアルゴリズム、wgpu テクスチャ管理）
- [x] sdit-render: cosmic-text `FontSystem` + `SwashCache` でグリフラスタライズ
- [x] sdit-render: `FontContext` 構造体（セルメトリクス計算・グリフキャッシュ）
- [x] テスト: アトラスへのグリフ配置・メトリクス計算

### Step 3: グリッドレンダリングパイプライン ✓ 完了（2026-03-11）
- [x] sdit-render: WGSL シェーダー（背景色 + テキスト描画）cell.wgsl
- [x] sdit-render: `CellPipeline` 構造体（wgpu パイプライン・バインドグループ）
- [x] sdit-render: Grid → `CellVertex` 変換（インスタンス描画方式）
- [x] sdit-render: `update_from_grid()` + `render_frame()` で GPU 描画
- [x] 静的テキスト "Hello, SDIT!" 描画の実装完了

#### セキュリティレビュー結果（Step 2+3）
- **Low**: `vertex_buffer` サイズが 80×24 固定のため、グリッドサイズが変更された場合に `write_buffer` 超過の可能性あり
  → Step 4（グリッドリサイズ実装時）に動的サイズ計算に変更する
- `unsafe_code = "deny"` 維持確認済み
- bytemuck::cast_slice は Pod/Zeroable derive で安全
- ペネトレーションテスト: PTY 接続（Step 4）まで不要

### Step 4: PTY スレッド接続 + Terminal 状態共有 ✓ 完了（2026-03-11）
- [x] sdit binary: `Arc<Mutex<TerminalState>>` で Terminal + Processor 状態共有
- [x] sdit binary: PTY reader スレッド（read → VTE parse → Terminal 更新）
- [x] sdit binary: PTY → `EventLoopProxy::send_event(PtyOutput)` で再描画要求
- [x] sdit binary: ウィンドウリサイズ → GPU resize + Terminal::resize()
- [x] CellPipeline::ensure_capacity() で動的バッファリサイズ

### Step 5: キー入力 → PTY 送信 ✓ 完了（2026-03-11）
- [x] sdit binary: winit `KeyEvent` → バイト列変換（APP_CURSOR 対応 + Ctrl+a-z）
- [x] sdit binary: Main → PTY writer チャネル（`mpsc::sync_channel(64)`）
- [x] TERM=xterm-256color 環境変数設定

### 完了処理 ✓ 完了（2026-03-11）
- [x] `cargo fmt --check && cargo clippy --all-targets && cargo test` — 71テスト通過、警告ゼロ
- [x] セキュリティレビュー: Medium 2件修正（atlas境界チェック、ensure_capacity オーバーフロー）
- [x] Plan ファイル更新済み
- [x] knowhow 記録・コミット
