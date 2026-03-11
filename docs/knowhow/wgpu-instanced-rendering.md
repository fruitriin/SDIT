# wgpu: インスタンス描画によるターミナルセルレンダリング

## 設計方針

各ターミナルセルを1インスタンスとして、`draw(0..6, 0..cell_count)` で描画する。

- 頂点数: 6（2 triangles）× セル数 = 1パスで全セル描画
- 頂点バッファのステップモード: `Instance`（頂点ごとではなくインスタンスごとにデータが進む）
- シェーダーで `vertex_index` (0-5) から quad の隅座標を生成

## CellVertex の設計

```rust
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CellVertex {
    pub bg: [f32; 4],           // 背景色 RGBA
    pub fg: [f32; 4],           // 前景色 RGBA
    pub grid_pos: [f32; 2],     // グリッド位置 (col, row)
    pub uv: [f32; 4],           // アトラス UV (min_u, min_v, max_u, max_v)
    pub glyph_offset: [f32; 2], // グリフオフセット (placement_left, placement_top)
    pub glyph_size: [f32; 2],   // グリフサイズ (width, height) px
}
```

## WGSL シェーダーの quad 生成

```wgsl
var QUAD_UV: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.0, 1.0),
    vec2(1.0, 0.0), vec2(1.0, 1.0), vec2(0.0, 1.0),
);
let corner = QUAD_UV[vertex_index];
let screen = grid_pos * cell_size + corner * cell_size;
let clip = vec2(screen.x / surface_size.x * 2.0 - 1.0,
               -(screen.y / surface_size.y * 2.0 - 1.0));
```

## バッファサイズの注意

- `vertex_buffer` は事前に最大セル数 × `sizeof(CellVertex)` で確保する
- 現状 80×24 固定。グリッドリサイズ時は動的に再確保する必要がある

## フォーマット互換

- アトラステクスチャ: `R8Unorm`
- サンプラー: `FilterMode::Linear`（グリフのボケ防止のため Nearest も検討）
- ブレンド: `ALPHA_BLENDING`（アルファ合成）

## wgpu 0.20 の API 差異

- `entry_point`: `Some("vs_main")` ではなく `"vs_main"` (参照型)
- `RenderPipelineDescriptor` に `cache` フィールドなし（0.20 時点）
