# Phase 0 — リファレンス読解

## 目的
リファレンスプロジェクト（Alacritty, Ghostty, WezTerm, Zellij）のソースを読み、
SDITの設計判断に必要な知見を蓄積する。

## 前提条件
- サブモジュール初期化済み: `git submodule update --init --depth=1`

## タスク
- [x] `refs/alacritty/alacritty-terminal/src/` を読んでグリッド設計を理解
- [x] `refs/ghostty/src/Surface.zig` を読んでサーフェス概念を理解
- [x] `refs/wezterm/wezterm-mux/src/` を読んでMux/SDI変換を設計
- [x] `refs/zellij/default-plugins/tab-bar/` を読んで縦タブUI設計

## 成果物
- `docs/ref-notes/` に読解メモを作成する（命名規約は CLAUDE.md 参照）

## 参照
- CLAUDE.md「参照指針：プロジェクト別」セクション
