# Phase 1 — sdit-core MVP

## 目的
ヘッドレスで動作するターミナルコアを実装する。
GUIなしでPTY起動・VTE処理・グリッド管理ができる状態をゴールとする。

## 前提条件
- Phase 0 のリファレンス読解が完了していること（最低限 Alacritty grid/pty/ansi）

## タスク
- [x] PTY起動・読み書き（Alacritty `tty/` 参照）— `crates/sdit-core/src/pty/mod.rs`
- [x] VTEパーサー統合（`vte` クレート使用、Alacritty `ansi.rs` 参照）— `crates/sdit-core/src/terminal/{mod,handler}.rs`
- [x] グリッドデータ構造（Alacritty `grid/` 参照）— `index.rs`, `grid/{cell,row,storage,mod}.rs`
- [x] ヘッドレステスト通過 — 61テスト（ユニット58 + 統合3）全通過

## 完了: 2026-03-11

## 対象クレート
- `crates/sdit-core/`

## 参照
- `refs/alacritty/alacritty-terminal/src/tty/`
- `refs/alacritty/alacritty-terminal/src/ansi.rs`
- `refs/alacritty/alacritty-terminal/src/grid/`
