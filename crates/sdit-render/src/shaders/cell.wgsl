// セル描画シェーダー
// 各ターミナルセルを 6 頂点（2 triangles）のインスタンスで描画する。
//
// 1 パスで背景 + テキストを描く:
//   - フラグメントシェーダーがグリフ領域内ならアトラスのアルファを読んで fg 色
//   - グリフ領域外なら bg 色

// ── Uniforms ──────────────────────────────────────────────────────────────

struct Uniforms {
    /// セルの幅・高さ（ピクセル）
    cell_size: vec2<f32>,
    /// グリッドの列数・行数
    grid_size: vec2<f32>,
    /// ウィンドウサイズ（ピクセル）
    surface_size: vec2<f32>,
    /// アトラステクスチャの一辺（ピクセル）
    atlas_size: f32,
    /// 描画開始 X オフセット（ピクセル）。サイドバー分のオフセットに使用。
    origin_x: f32,
}

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var atlas_tex: texture_2d<f32>;
@group(0) @binding(2) var atlas_sampler: sampler;

// ── Per-instance vertex inputs ────────────────────────────────────────────

struct CellInput {
    /// 背景色 RGBA (0.0–1.0)
    @location(0) bg: vec4<f32>,
    /// 前景色 RGBA (0.0–1.0)
    @location(1) fg: vec4<f32>,
    /// グリッド位置 (column, row) — 0-based
    @location(2) grid_pos: vec2<f32>,
    /// アトラス UV (u_min, v_min, u_max, v_max)
    @location(3) uv: vec4<f32>,
    /// グリフオフセット (x, y) — ベースラインからの pixel オフセット
    @location(4) glyph_offset: vec2<f32>,
    /// グリフサイズ (width, height) — ピクセル
    @location(5) glyph_size: vec2<f32>,
    /// セル幅の倍率（通常 1.0、全角文字 2.0）
    @location(6) cell_width_scale: f32,
}

// ── Vertex output ─────────────────────────────────────────────────────────

struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    /// セル左上を原点としたローカル座標（ピクセル）
    @location(0) local_pos: vec2<f32>,
    /// 背景色
    @location(1) bg: vec4<f32>,
    /// 前景色
    @location(2) fg: vec4<f32>,
    /// アトラス UV rect (u_min, v_min, u_max, v_max)
    @location(3) uv: vec4<f32>,
    /// グリフ開始ローカル座標（ピクセル）
    @location(4) glyph_start: vec2<f32>,
    /// グリフサイズ（ピクセル）
    @location(5) glyph_size: vec2<f32>,
}

// ── Vertex shader ─────────────────────────────────────────────────────────

// 頂点 0–5 で quad を生成する（2 triangles, CCW winding）。
//
//   0--1
//   |  |   → triangles: (0,1,2) と (1,3,2)
//   2--3
//
// vertex_index を 2 ビット (col, row) にデコードして隅を選ぶ。

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    instance: CellInput,
) -> VsOut {
    // quad の隅インデックス: 0,1,2  /  1,3,2
    var QUAD_UV: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 1.0),
    );

    let corner = QUAD_UV[vertex_index];

    // セル左上のピクセル座標。
    let cell_px = instance.grid_pos * u.cell_size;

    // セル内ローカル座標（ピクセル）。全角文字は横方向2セル分に拡張。
    let safe_scale = clamp(instance.cell_width_scale, 1.0, 2.0);
    let scaled_cell = vec2<f32>(u.cell_size.x * safe_scale, u.cell_size.y);
    let local = corner * scaled_cell;

    // スクリーン座標（Y は下向きが正）。origin_x でサイドバー分オフセット。
    let screen = cell_px + local + vec2<f32>(u.origin_x, 0.0);

    // クリップ空間に変換（-1..+1, Y 反転）。
    let clip = vec2<f32>(
        screen.x / u.surface_size.x * 2.0 - 1.0,
        -(screen.y / u.surface_size.y * 2.0 - 1.0),
    );

    // ベースライン Y = cell_px.y + baseline_offset
    // baseline_offset は glyph_offset.y の符号を逆に解釈して渡す。
    // ここでは glyph_start = (baseline からの left, top) として渡す。
    // セルのベースラインを cell_height * 0.8 に設定する（font.rs と合わせる）。
    let baseline_y = u.cell_size.y * 0.8;
    let glyph_start = vec2<f32>(
        instance.glyph_offset.x,
        baseline_y - instance.glyph_offset.y,
    );

    var out: VsOut;
    out.clip_pos = vec4<f32>(clip, 0.0, 1.0);
    out.local_pos = local;
    out.bg = instance.bg;
    out.fg = instance.fg;
    out.uv = instance.uv;
    out.glyph_start = glyph_start;
    out.glyph_size = instance.glyph_size;
    return out;
}

// ── Fragment shader ───────────────────────────────────────────────────────

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let glyph_end = in.glyph_start + in.glyph_size;

    // ピクセルがグリフ矩形内にあるか。
    let in_glyph = all(in.local_pos >= in.glyph_start) && all(in.local_pos < glyph_end);

    if !in_glyph || in.glyph_size.x <= 0.0 || in.glyph_size.y <= 0.0 {
        return in.bg;
    }

    // グリフ内のローカル UV（0–1）。
    let glyph_local = (in.local_pos - in.glyph_start) / in.glyph_size;

    // アトラス UV を補間。
    let atlas_uv = vec2<f32>(
        mix(in.uv.x, in.uv.z, glyph_local.x),
        mix(in.uv.y, in.uv.w, glyph_local.y),
    );

    let alpha = textureSample(atlas_tex, atlas_sampler, atlas_uv).r;

    // アルファブレンド: bg と fg を alpha で混ぜる。
    return mix(in.bg, in.fg, alpha);
}
