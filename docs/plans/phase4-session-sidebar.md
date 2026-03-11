# Phase 4 — 縦タブ（SessionSidebar）

## 目的
セッションが2つ以上になったときに縦タブバー（SessionSidebar）を表示し、
Chrome-like なタブ合体・切出しUXを実現する。

## 前提条件
- Phase 3 のSDI複数ウィンドウが完了していること

## タスク
- [ ] Cmd+\ でサイドバートグル
- [ ] セッション一覧の表示（Zellij `tab-bar` 参照）
- [ ] サイドバーからウィンドウフォーカス
- [ ] セッション追加・削除

## 対象クレート
- `crates/sdit-session/` (`sidebar.rs`)
- `crates/sdit/`

## 参照
- `refs/zellij/default-plugins/tab-bar/src/`
- CLAUDE.md「縦タブへの適用」セクション
