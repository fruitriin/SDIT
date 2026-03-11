# Phase 3 — SDI本実装

## 目的
複数のSDIウィンドウを同時に動作させ、
各ウィンドウが独立したセッションを持つ状態を実現する。

## 前提条件
- Phase 2 の単一ウィンドウ表示が完了していること

## タスク
- [ ] 複数ウィンドウ（複数セッション）の同時動作
- [ ] ウィンドウタイトルバー（セッション色・cwd表示）
- [ ] WezTerm `glwindow.rs` 参照でウィンドウライフサイクル実装

## 対象クレート
- `crates/sdit/`
- `crates/sdit-session/`

## 参照
- `refs/wezterm/wezterm-gui/src/glwindow.rs`
- `refs/wezterm/wezterm-mux/src/`
