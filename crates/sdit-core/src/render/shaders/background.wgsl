// 背景画像シェーダー
// ウィンドウ全体に背景画像を描画する。フルスクリーンクワッド（vertex_index 0-5）。

struct BgUniforms {
    /// 表示モード: 0=contain, 1=cover, 2=fill
    fit_mode: u32,
    /// 不透明度 (0.0-1.0)
    opacity: f32,
    /// ウィンドウサイズ（ピクセル）
    surface_size: vec2<f32>,
    /// 画像サイズ（ピクセル）
    image_size: vec2<f32>,
}

@group(0) @binding(0) var<uniform> u: BgUniforms;
@group(0) @binding(1) var bg_tex: texture_2d<f32>;
@group(0) @binding(2) var bg_sampler: sampler;

struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VsOut {
    var CORNERS: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0, -1.0),
    );

    // UV: clip (-1..+1) → screen (0..1)
    var UVS: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 1.0),
    );

    var out: VsOut;
    out.clip_pos = vec4<f32>(CORNERS[vertex_index], 0.0, 1.0);
    out.uv = UVS[vertex_index];
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    // フィットモードに応じて UV を調整する。
    var uv = in.uv;

    let surf_w = u.surface_size.x;
    let surf_h = u.surface_size.y;
    let img_w = u.image_size.x;
    let img_h = u.image_size.y;

    if img_w <= 0.0 || img_h <= 0.0 {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    let surf_aspect = surf_w / surf_h;
    let img_aspect  = img_w  / img_h;

    if u.fit_mode == 0u {
        // contain: 画像全体が見えるようにする
        // スケールは min(surf/img)
        var scale_x: f32;
        var scale_y: f32;
        if surf_aspect > img_aspect {
            // ウィンドウのほうが横長: 高さに合わせる
            scale_y = 1.0;
            scale_x = img_aspect / surf_aspect;
        } else {
            // 画像のほうが横長: 幅に合わせる
            scale_x = 1.0;
            scale_y = surf_aspect / img_aspect;
        }
        // 中央配置
        let offset = vec2<f32>((1.0 - scale_x) * 0.5, (1.0 - scale_y) * 0.5);
        let tex_uv = (in.uv - offset) / vec2<f32>(scale_x, scale_y);
        // 範囲外なら透明
        if tex_uv.x < 0.0 || tex_uv.x > 1.0 || tex_uv.y < 0.0 || tex_uv.y > 1.0 {
            return vec4<f32>(0.0, 0.0, 0.0, 0.0);
        }
        uv = tex_uv;
    } else if u.fit_mode == 1u {
        // cover: ウィンドウ全体を覆う
        var scale_x: f32;
        var scale_y: f32;
        if surf_aspect > img_aspect {
            // ウィンドウのほうが横長: 幅に合わせる
            scale_x = 1.0;
            scale_y = surf_aspect / img_aspect;
        } else {
            // 画像のほうが横長: 高さに合わせる
            scale_y = 1.0;
            scale_x = img_aspect / surf_aspect;
        }
        // 中央配置
        let offset = vec2<f32>((1.0 - scale_x) * 0.5, (1.0 - scale_y) * 0.5);
        uv = offset + in.uv * vec2<f32>(scale_x, scale_y);
    }
    // fill (fit_mode == 2): uv はそのまま（全体に引き伸ばす）

    let color = textureSample(bg_tex, bg_sampler, uv);
    return vec4<f32>(color.rgb, color.a * u.opacity);
}
