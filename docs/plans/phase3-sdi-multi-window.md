# Phase 3 — SDI本実装

## 目的
複数のSDIウィンドウを同時に動作させ、
各ウィンドウが独立したセッションを持つ状態を実現する。

## 前提条件
- Phase 2 の単一ウィンドウ表示が完了していること

## 設計方針

### Session ≠ Window 分離（WezTerm Mux 参照）
- Session = PTY + Terminal 状態。ウィンドウとは独立して生存する
- Window = GUI ウィンドウ。Session を1つ表示する
- 合体・切出し時（Phase 4）は Window の参照先 Session を差し替えるだけ

### スレッドモデル（pty-threading-model.md 参照）
- Session ごとに PTY Reader/Writer スレッドを持つ（現行3スレッドモデルの拡張）
- TerminalState は `Arc<Mutex<>>` で保護（現行と同じ）

### ウィンドウライフサイクル
- 各ウィンドウは独自の GPU コンテキスト（Surface + Pipeline + Atlas）を持つ
- FontContext は全ウィンドウで共有可能（読み取り専用）

## タスク

### 3.1 sdit-session クレート実装
- [x] `Session` 型: SessionId, TerminalState(Terminal + Processor), PTY I/O チャネル
- [x] `SessionManager` 型: Session の CRUD、ID 採番、ライフサイクル管理

### 3.2 main.rs 複数ウィンドウ対応リファクタ
- [x] `WindowState` 型: WindowId ごとの GPU コンテキスト・パイプライン・SessionId
- [x] `SditApp` を `HashMap<WindowId, WindowState>` ベースに変更
- [x] `window_event()` で WindowId に基づいて正しい WindowState を取得
- [x] `user_event(PtyOutput)` に SessionId を含めて正しいウィンドウを再描画

### 3.3 新規ウィンドウ生成
- [x] Cmd+N (macOS) / Ctrl+Shift+N で新規ウィンドウ + 新規セッション生成
- [x] `create_window()` メソッド: Window 生成 → GPU 初期化 → Session 生成 → PTY 起動

### 3.4 ウィンドウ・セッション終了
- [x] CloseRequested → そのウィンドウの Session のみ終了（他ウィンドウは維持）
- [x] ChildExit → 対応する Session のウィンドウを閉じる
- [x] 全ウィンドウ閉じ → event_loop.exit()

### 3.5 テスト
- [x] `cargo test` 全通過（既存テスト退行なし）
- [x] `scripts/check.sh` 通過

## 対象クレート
- `crates/sdit-session/` — Session, SessionManager
- `crates/sdit/` — main.rs リファクタ

## 参照
- `refs/wezterm/wezterm-gui/src/glwindow.rs` — ウィンドウライフサイクル
- `refs/wezterm/wezterm-mux/src/` — Session 管理
- `docs/knowhow/architecture-decisions.md` — 3層分離アーキテクチャ
- `docs/knowhow/pty-threading-model.md` — スレッドモデル
- `docs/knowhow/wgpu-winit-integration.md` — wgpu+winit 統合

## 完了条件
- [x] 複数のSDIウィンドウが同時に動作する
- [x] 各ウィンドウが独立したセッション（PTY）を持つ
- [x] 1つのウィンドウを閉じても他のウィンドウは動作し続ける
- [x] 全ウィンドウを閉じるとアプリが終了する
- [x] Cmd+N で新規ウィンドウを生成できる
- [x] `cargo test` + `scripts/check.sh` 全通過
- [x] セキュリティレビュー完了（M-3修正済み、M-1/M-2は独立計画）

## セキュリティレビュー結果

| ID | 重要度 | 概要 | 対応 |
|---|---|---|---|
| M-1 | Medium | PTY リサイズ時に SIGWINCH 未送信 | `phase3.1-security-fixes.md` で独立計画 |
| M-2 | Medium | Session 削除時にスレッドが join されない | `phase3.1-security-fixes.md` で独立計画 |
| M-3 | Medium | SessionId u64 オーバーフロー未チェック | **修正済み** (`checked_add` に変更) |
| L-1 | Low | Mutex poisoning を黙って継続 | 記録（既存の knowhow に記載済み。将来対応） |
| L-2 | Low | ウィンドウ無限生成の制限なし | 記録（Phase 5 で設定可能にする） |
| L-3 | Low | スレッド spawn の unwrap パニック | 記録（Phase 3.1 で M-2 と一緒に対応検討） |
| L-4 | Low | ChildExit イベント送信失敗の無視 | 記録（影響限定的） |
| I-1 | Info | modifiers 状態のウィンドウ間共有 | 記録 |
| I-2 | Info | 極小ウィンドウサイズの考慮不足 | 記録 |
