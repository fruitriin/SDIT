# Phase 2 — 最初のSDIウィンドウ

## 目的
winit + wgpu で1枚のウィンドウを表示し、
PTYの出力をGPUレンダリングで描画する最小構成を実現する。

## 前提条件
- Phase 1 の sdit-core MVP が完了していること

## タスク
- [x] winit + wgpu で1ウィンドウ表示 ← **Step 1 完了**
- [ ] グリッドをテクスチャアトラスでレンダリング（Ghostty参照）
- [ ] PTYとウィンドウを接続
- [ ] キー入力をPTYに送信
  ~~OSC タイトル上限・Line型安全性は Phase 1 で修正済み~~

## Step 1 実装記録

### 実装内容（2026-03-11）

**`crates/sdit-render/src/pipeline.rs`**:
- `GpuContext<'window>` 構造体を実装（device, queue, surface, surface_config を保持）
- `GpuContext::new(&Arc<Window>) -> Result<GpuContext<'static>>`: pollster::block_on で wgpu の async API を同期呼び出し
- `GpuContext::resize(u32, u32)`: 0サイズスキップ付きリサイズ
- `GpuContext::render_frame()`: Catppuccin Mocha base (#1e1e2e) でクリアするだけの最小レンダリング

**`crates/sdit/src/main.rs`**:
- `SditEvent` カスタムイベント型（後の Phase で拡張）
- `SditApp` + `ApplicationHandler<SditEvent>` 実装
- `resumed`: ウィンドウ生成・GpuContext 初期化
- `window_event`: CloseRequested/Resized/RedrawRequested/SurfaceError::Lost|Outdated ハンドリング
- `about_to_wait`: request_redraw() で連続描画

**依存追加**:
- `pollster = "0.4"` をワークスペース依存に追加
- `sdit-render/Cargo.toml` に `winit`, `anyhow`, `pollster` を追加

**ライフタイム設計**:
- `GpuContext<'window>` のライフタイムパラメータは構造体定義上残しつつ、`new()` は `GpuContext<'static>` を返す
- `Arc<Window>` が Surface の生存期間を保証するため 'static は安全
- `unsafe` は一切使用していない

### セキュリティレビュー結果
- 外部入力に触れるコードなし（Surface エラーハンドリングのみ）
- wgpu のバリデーション層が入力バウンダリを担保
- `unsafe_code = "deny"` 維持確認

## 対象クレート
- `crates/sdit/` (バイナリ)
- `crates/sdit-render/`

## 参照
- `refs/ghostty/src/renderer/`
- `refs/ghostty/src/Surface.zig`
- `refs/alacritty/alacritty/src/event.rs`
