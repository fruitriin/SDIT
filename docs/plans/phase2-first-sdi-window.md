# Phase 2 — 最初のSDIウィンドウ

## 目的
winit + wgpu で1枚のウィンドウを表示し、
PTYの出力をGPUレンダリングで描画する最小構成を実現する。

## 前提条件
- Phase 1 の sdit-core MVP が完了していること

## タスク
- [ ] winit + wgpu で1ウィンドウ表示
- [ ] グリッドをテクスチャアトラスでレンダリング（Ghostty参照）
- [ ] PTYとウィンドウを接続
- [ ] キー入力をPTYに送信
- [ ] OSC タイトル文字列に長さ上限（4096バイト）を設ける（Phase 1 セキュリティレビュー指摘）
- [ ] `Line(i32)` の型安全性を検討（Phase 1 セキュリティレビュー指摘）

## 対象クレート
- `crates/sdit/` (バイナリ)
- `crates/sdit-render/`

## 参照
- `refs/ghostty/src/renderer/`
- `refs/ghostty/src/Surface.zig`
- `refs/alacritty/alacritty/src/event.rs`
