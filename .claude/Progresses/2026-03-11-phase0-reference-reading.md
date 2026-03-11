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
4. Lint・ビルドを通す
5. 投入されたタスクのPlanに実装完了状況を反映する
6. .claude/Feedback.md にPlan, TODO, Progress推進エンジンの問題の記録・改善アクションを追記する。反映済みの項目は削除する
7. .claude/Feedback.md にプロジェクト進行上の問題の記録・改善アクションを追記する。反映済みの項目は削除する
8. `.claude/Progresses/YYYY-MM-DD-プラン名.md` にリネームして移動し、`.claude/templates/ProgressTemplate.md` から新規の Progress.md を作成する
9. 実装の知見で継続して効果が見込めるもの、再調査が必要なものを docs/knowhow に.mdで作成する。

10. コミットする

---

## タスク: Phase 0 — リファレンス読解

### 1. Alacritty — グリッド設計の理解
- [x] `alacritty-terminal/src/grid/` のデータ構造を読解
- [x] `alacritty-terminal/src/ansi.rs` VTEパーサー統合を読解
- [x] `alacritty-terminal/src/tty/` PTYプロセス管理を読解
- [x] `docs/ref-notes/alacritty-grid.md` 読解メモ作成

### 2. Ghostty — サーフェス概念の理解
- [x] `src/Surface.zig` サーフェス管理を読解
- [x] `src/App.zig` アプリケーション構造を読解
- [x] `src/terminal/Terminal.zig` ターミナルステートを読解
- [x] `docs/ref-notes/ghostty-surface.md` 読解メモ作成

### 3. WezTerm — Mux/SDI変換の設計
- [x] `wezterm-mux/src/` セッション多重化を読解
- [x] `wezterm-gui/src/glwindow.rs` ウィンドウライフサイクルを読解
- [x] `docs/ref-notes/wezterm-mux.md` 読解メモ作成

### 4. Zellij — 縦タブUI設計
- [x] `default-plugins/tab-bar/src/` タブバープラグインを読解
- [x] `zellij-utils/src/data.rs` セッション状態型を読解
- [x] `docs/ref-notes/zellij-tabbar.md` 読解メモ作成
