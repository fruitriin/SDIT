# Phase 2 — 最初のSDIウィンドウ

## 目的
winit + wgpu で1枚のウィンドウを表示し、
PTYの出力をGPUレンダリングで描画する最小構成を実現する。

## 前提条件
- Phase 1 の sdit-core MVP が完了していること

## タスク
- [x] winit + wgpu で1ウィンドウ表示 ← **Step 1 完了**
- [x] グリッドをテクスチャアトラスでレンダリング ← **Step 2+3 完了**
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

## Step 2+3 実装記録（2026-03-11）

### 実装内容

**`crates/sdit-render/src/atlas.rs`**:
- `Atlas` 構造体: シェルフアルゴリズムで R8Unorm テクスチャアトラスを管理
- `AtlasRegion`: グリフ配置領域の座標（UV計算用）
- `reserve(w, h) -> Option<AtlasRegion>`: シェルフへの配置（新シェルフ自動作成）
- `write(region, data)`: CPU バッファへの書き込み
- `upload_if_dirty(queue)`: dirty フラグが立っているときのみ GPU に転送
- ユニットテスト4件（インメモリダミーで wgpu 不要）

**`crates/sdit-render/src/font.rs`**:
- `FontContext` 構造体: FontSystem + SwashCache + グリフキャッシュを管理
- `CellMetrics`: cell_width / cell_height / baseline / font_size
- `rasterize_glyph(c, atlas)`: Buffer でシェーピング → SwashCache でラスタライズ → Atlas に配置
- セル幅計算: モノスペースフォントの `monospace_em_width()` × font_size
- ユニットテスト2件（メトリクス妥当性確認）

**`crates/sdit-render/src/shaders/cell.wgsl`**:
- `Uniforms`: cell_size / grid_size / surface_size / atlas_size
- `CellInput`: per-instance (bg, fg, grid_pos, uv, glyph_offset, glyph_size)
- 頂点シェーダー: vertex_index 0-5 で quad を生成、グリッド座標→クリップ座標変換
- フラグメントシェーダー: グリフ領域内ならアトラスからアルファをサンプリングして fg/bg をブレンド

**`crates/sdit-render/src/pipeline.rs`**（追加分）:
- `CellVertex`: bytemuck::Pod/Zeroable で GPU インスタンスデータ
- `CellPipeline`: wgpu レンダーパイプライン + バインドグループ + 頂点バッファ
- `GpuContext::render_frame()`: CellPipeline を受け取りインスタンス描画
- `update_from_grid()`: Grid → CellVertex 変換 + GPU バッファ更新
- カラー変換: NamedColor/Indexed/Rgb → RGBA（Catppuccin Mocha パレット）

**`crates/sdit/src/main.rs`**（更新）:
- `init_render()`: Grid 作成 → FontContext → Atlas → CellPipeline 初期化
- `write_str_to_grid()`: 文字列を Grid セルに書き込むヘルパー
- "Hello, SDIT!" を Grid の (0, 0) から書き込んで描画

### セキュリティレビュー結果
- **Low**: `vertex_buffer` サイズが 80×24 固定。Step 4 のグリッドリサイズ時に動的サイズ計算に変更する
- `unsafe_code = "deny"` 維持確認済み
- bytemuck は Pod/Zeroable derive マクロで安全に使用
- ペネトレーションテスト: Step 4（PTY接続）まで不要

## 対象クレート
- `crates/sdit/` (バイナリ)
- `crates/sdit-render/`

## 参照
- `refs/ghostty/src/renderer/`
- `refs/ghostty/src/Surface.zig`
- `refs/alacritty/alacritty/src/event.rs`
