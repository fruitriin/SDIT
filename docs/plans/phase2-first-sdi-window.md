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

## 対象クレート
- `crates/sdit/` (バイナリ)
- `crates/sdit-render/`

## Phase 1 からの引継ぎセキュリティ項目
- [ ] OSC タイトル文字列に長さ上限（4096バイト）を設ける
- [ ] `Line(i32)` の型安全性を検討（viewport 内は `u32` に変更 or 型レベル非負保証）

## 参照
- `refs/ghostty/src/renderer/`
- `refs/ghostty/src/Surface.zig`
- `refs/alacritty/alacritty/src/event.rs`
