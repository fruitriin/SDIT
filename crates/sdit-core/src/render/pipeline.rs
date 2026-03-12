//! wgpu パイプライン。
//!
//! - `GpuContext`: Surface / Device / Queue の初期化と管理
//! - `CellVertex`: 1セル分のインスタンスデータ
//! - `CellPipeline`: ターミナルグリッドをセル単位でレンダリングするパイプライン

use std::sync::Arc;

use crate::grid::Grid;
use crate::grid::{Cell, CellFlags, Color, Dimensions, NamedColor};
use crate::index::{Column, Line, Point};
use anyhow::{Context as _, Result};
use bytemuck::{Pod, Zeroable};
use winit::window::Window;

use super::atlas::Atlas;
use super::font::FontContext;

// ---------------------------------------------------------------------------
// GpuContext
// ---------------------------------------------------------------------------

/// GPU コンテキスト。Surface・Device・Queue をまとめて保持する。
pub struct GpuContext<'window> {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'window>,
    pub surface_config: wgpu::SurfaceConfiguration,
}

impl GpuContext<'_> {
    /// Arc<Window> からGPUコンテキストを初期化する。
    pub fn new(window: &Arc<Window>) -> Result<GpuContext<'static>> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface =
            instance.create_surface(Arc::clone(window)).context("wgpu Surface 作成失敗")?;

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .context("wgpu Adapter 取得失敗")?;

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("sdit-render device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
            },
            None,
        ))
        .context("wgpu Device 取得失敗")?;

        let size = window.inner_size();
        let caps = surface.get_capabilities(&adapter);
        let format = caps.formats.first().copied().context("対応サーフェスフォーマットなし")?;

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes.first().copied().unwrap_or(wgpu::CompositeAlphaMode::Auto),
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        Ok(GpuContext { device, queue, surface, surface_config })
    }

    /// ウィンドウリサイズ時にサーフェスを再設定する。
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
    }

    /// フレームを1枚レンダリングして present する。
    ///
    /// `pipelines` 内の各パイプラインを順番に描画する。
    /// サイドバー + ターミナルの2パス描画に対応。
    pub fn render_frame(
        &self,
        pipelines: &[&CellPipeline],
        clear_color: [f32; 4],
    ) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("sdit frame encoder"),
        });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("sdit clear pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: f64::from(clear_color[0]),
                            g: f64::from(clear_color[1]),
                            b: f64::from(clear_color[2]),
                            a: f64::from(clear_color[3]),
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            for cp in pipelines {
                if cp.cell_count > 0 {
                    pass.set_pipeline(&cp.pipeline);
                    pass.set_bind_group(0, &cp.bind_group, &[]);
                    pass.set_vertex_buffer(0, cp.vertex_buffer.slice(..));
                    pass.draw(0..6, 0..cp.cell_count);
                }
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// CellVertex — per-instance GPU data
// ---------------------------------------------------------------------------

/// セル1つの GPU インスタンスデータ。
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct CellVertex {
    /// 背景色 RGBA (0.0–1.0)
    pub bg: [f32; 4],
    /// 前景色 RGBA (0.0–1.0)
    pub fg: [f32; 4],
    /// グリッド位置 (column, row)
    pub grid_pos: [f32; 2],
    /// アトラス UV (`u_min`, `v_min`, `u_max`, `v_max`)
    pub uv: [f32; 4],
    /// グリフオフセット (`placement_left`, `placement_top`)
    pub glyph_offset: [f32; 2],
    /// グリフサイズ (width, height) in pixels
    pub glyph_size: [f32; 2],
    /// セル幅の倍率。通常 1.0、全角文字は 2.0。
    pub cell_width_scale: f32,
}

impl CellVertex {
    /// wgpu 頂点バッファレイアウト（インスタンスステップモード）。
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        use wgpu::VertexFormat as F;
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute { shader_location: 0, offset: 0, format: F::Float32x4 },
                wgpu::VertexAttribute { shader_location: 1, offset: 16, format: F::Float32x4 },
                wgpu::VertexAttribute { shader_location: 2, offset: 32, format: F::Float32x2 },
                wgpu::VertexAttribute { shader_location: 3, offset: 40, format: F::Float32x4 },
                wgpu::VertexAttribute { shader_location: 4, offset: 56, format: F::Float32x2 },
                wgpu::VertexAttribute { shader_location: 5, offset: 64, format: F::Float32x2 },
                wgpu::VertexAttribute { shader_location: 6, offset: 72, format: F::Float32 },
            ],
        }
    }
}

// ---------------------------------------------------------------------------
// Uniforms
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Uniforms {
    cell_size: [f32; 2],
    grid_size: [f32; 2],
    surface_size: [f32; 2],
    atlas_size: f32,
    /// 描画開始 X オフセット（ピクセル）。サイドバー分のオフセットに使用。
    origin_x: f32,
}

// ---------------------------------------------------------------------------
// CellPipeline
// ---------------------------------------------------------------------------

/// ターミナルセルをレンダリングするパイプライン。
pub struct CellPipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    /// 描画するセル数。
    pub cell_count: u32,
    /// 頂点バッファの確保容量（セル数）。
    vertex_buffer_capacity: usize,
}

impl CellPipeline {
    /// 新しい `CellPipeline` を作成する。
    ///
    /// - `surface_format`: サーフェスのカラーフォーマット
    /// - `atlas`: テクスチャアトラス（既に GPU にアップロード済みであること）
    /// - `initial_capacity`: 頂点バッファの初期容量（セル数）
    ///
    /// Uniforms は作成後に `update_uniforms()` で設定すること。
    #[allow(clippy::too_many_lines)]
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        atlas: &Atlas,
        initial_capacity: usize,
    ) -> Self {
        let shader_source = include_str!("shaders/cell.wgsl");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cell shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        // バインドグループレイアウト: uniform + texture + sampler
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("cell bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cell uniform buffer"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        // 初期値を書き込む。
        // NOTE: queue が必要なため、new() の後に update_uniforms() で書き込む設計にする。
        // ここではバッファのみ確保する。

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("atlas sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("cell bind group"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: uniform_buffer.as_entire_binding() },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(atlas.texture_view()),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("cell pipeline layout"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("cell pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[CellVertex::layout()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        // 頂点バッファ: 指定されたセル数で初期確保。
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cell vertex buffer"),
            size: (initial_capacity.max(1) * std::mem::size_of::<CellVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            bind_group,
            vertex_buffer,
            uniform_buffer,
            cell_count: 0,
            vertex_buffer_capacity: initial_capacity.max(1),
        }
    }

    /// GPU バッファサイズ上限: 64MB（異常なグリッドサイズへの防御）。
    const MAX_BUFFER_BYTES: usize = 64 * 1024 * 1024;

    /// 頂点バッファの容量が `needed` 未満なら再確保する。
    pub fn ensure_capacity(&mut self, device: &wgpu::Device, needed: usize) {
        if needed <= self.vertex_buffer_capacity {
            return;
        }
        // 必要量の 2 倍に拡張してリアロケーションを減らす。
        let cell_size = std::mem::size_of::<CellVertex>();
        let max_cells = Self::MAX_BUFFER_BYTES / cell_size;
        let new_capacity = needed.saturating_mul(2).min(max_cells).max(needed);
        let buffer_size = new_capacity.saturating_mul(cell_size);
        self.vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cell vertex buffer"),
            size: buffer_size as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.vertex_buffer_capacity = new_capacity;
    }

    /// Uniforms を更新する。
    pub fn update_uniforms(
        &self,
        queue: &wgpu::Queue,
        cell_size: [f32; 2],
        grid_size: [f32; 2],
        surface_size: [f32; 2],
        atlas_size: f32,
        origin_x: f32,
    ) {
        let uniforms = Uniforms { cell_size, grid_size, surface_size, atlas_size, origin_x };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));
    }

    /// Grid からセルデータを構築して頂点バッファを更新する。
    ///
    /// `cursor_pos` はカーソル位置 `(column, row)` で、該当セルの fg/bg を反転描画する。
    /// `selection` は選択範囲 `(start, end)` で、範囲内のセルの fg/bg を反転描画する。
    #[allow(clippy::too_many_arguments)]
    pub fn update_from_grid(
        &mut self,
        queue: &wgpu::Queue,
        grid: &Grid<Cell>,
        font_ctx: &mut FontContext,
        atlas: &mut Atlas,
        atlas_size: f32,
        cell_size: [f32; 2],
        surface_size: [f32; 2],
        cursor_pos: Option<(usize, usize)>,
        selection: Option<((usize, usize), (usize, usize))>,
    ) {
        let rows = grid.screen_lines();
        let cols = grid.columns();

        // Uniforms を更新。origin_x は呼び出し側で設定する場合もあるが、
        // update_from_grid では常に 0.0（サイドバーオフセットなし）を使用する。
        self.update_uniforms(
            queue,
            cell_size,
            [cols as f32, rows as f32],
            surface_size,
            atlas_size,
            0.0,
        );

        let mut vertices: Vec<CellVertex> = Vec::with_capacity(rows * cols);

        for row in 0..rows {
            for col in 0..cols {
                // row は screen_lines() の範囲内なので i32 に収まる（最大 65535 程度）。
                #[allow(clippy::cast_possible_wrap)]
                let point = Point::new(Line(row as i32), Column(col));
                let cell = &grid[point];

                // WIDE_CHAR_SPACER（全角文字の右半分）は背景のみ描画。
                if cell.flags.contains(CellFlags::WIDE_CHAR_SPACER) {
                    vertices.push(CellVertex {
                        bg: color_to_rgba(cell.bg),
                        fg: [0.0; 4],
                        grid_pos: [col as f32, row as f32],
                        uv: [0.0; 4],
                        glyph_offset: [0.0; 2],
                        glyph_size: [0.0; 2],
                        cell_width_scale: 1.0,
                    });
                    continue;
                }

                let is_cursor = cursor_pos == Some((col, row));
                let is_selected = selection.is_some_and(|sel| is_in_selection(col, row, sel));
                let (bg, fg) = if is_cursor || is_selected {
                    // カーソル位置 or 選択範囲: fg/bg を反転
                    (color_to_rgba(cell.fg), color_to_rgba(cell.bg))
                } else {
                    (color_to_rgba(cell.bg), color_to_rgba(cell.fg))
                };
                let is_wide = cell.flags.contains(CellFlags::WIDE_CHAR);

                // グリフをラスタライズしてアトラスに配置。
                let (uv, glyph_offset, glyph_size) =
                    if let Some(entry) = font_ctx.rasterize_glyph(cell.c, atlas) {
                        let r = entry.region;
                        let uv = [
                            r.x as f32 / atlas_size,
                            r.y as f32 / atlas_size,
                            (r.x + r.width) as f32 / atlas_size,
                            (r.y + r.height) as f32 / atlas_size,
                        ];
                        let offset = [entry.placement_left as f32, entry.placement_top as f32];
                        let size = [r.width as f32, r.height as f32];
                        (uv, offset, size)
                    } else {
                        // スペース or グリフなし: ゼロサイズで背景のみ描画。
                        ([0.0_f32; 4], [0.0_f32; 2], [0.0_f32; 2])
                    };

                // WIDE_CHAR は2セル幅で描画。
                let cell_width_scale = if is_wide { 2.0 } else { 1.0 };
                vertices.push(CellVertex {
                    bg,
                    fg,
                    grid_pos: [col as f32, row as f32],
                    uv,
                    glyph_offset,
                    glyph_size,
                    cell_width_scale,
                });
            }
        }

        // アトラスをアップロード。
        atlas.upload_if_dirty(queue);

        let count = vertices.len();
        if count > 0 {
            queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        }
        self.cell_count = count as u32;
    }

    /// 生の `CellVertex` 列から頂点バッファを更新する。
    ///
    /// サイドバー等、Grid を介さないカスタム描画に使用する。
    pub fn update_cells(&mut self, queue: &wgpu::Queue, cells: &[CellVertex]) {
        let count = cells.len();
        if count > 0 {
            queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(cells));
        }
        self.cell_count = count as u32;
    }

    /// 頂点バッファの指定インデックスのセルを1つ上書きする。
    ///
    /// プリエディット描画など、グリッドの一部だけを上書きしたい場合に使用する。
    /// `index` が `cell_count` 以上の場合は何もしない（範囲外書き込み防止）。
    pub fn overwrite_cell(&self, queue: &wgpu::Queue, index: usize, vertex: &CellVertex) {
        if index as u32 >= self.cell_count {
            return;
        }
        let byte_offset = (index * std::mem::size_of::<CellVertex>()) as wgpu::BufferAddress;
        queue.write_buffer(&self.vertex_buffer, byte_offset, bytemuck::bytes_of(vertex));
    }
}

// ---------------------------------------------------------------------------
// カラー変換ヘルパー
// ---------------------------------------------------------------------------

/// セル (col, row) が選択範囲内かどうか判定する。
///
/// 選択範囲は行優先で正規化し、複数行にまたがる場合は
/// 開始行は開始列以降、中間行は全列、終了行は終了列以前が対象。
fn is_in_selection(col: usize, row: usize, selection: ((usize, usize), (usize, usize))) -> bool {
    let ((sc, sr), (ec, er)) = selection;
    // 正規化: start が end より前になるようにする
    let (start_col, start_row, end_col, end_row) =
        if (sr, sc) <= (er, ec) { (sc, sr, ec, er) } else { (ec, er, sc, sr) };

    if row < start_row || row > end_row {
        return false;
    }
    if start_row == end_row {
        // 同一行: 列範囲内
        return col >= start_col && col <= end_col;
    }
    if row == start_row {
        return col >= start_col;
    }
    if row == end_row {
        return col <= end_col;
    }
    // 中間行: 全列選択
    true
}

/// `Color` を RGBA `[f32; 4]` に変換する。
/// Named カラーは Catppuccin Mocha パレットにマッピングする。
fn color_to_rgba(color: Color) -> [f32; 4] {
    match color {
        Color::Rgb { r, g, b } => {
            [f32::from(r) / 255.0, f32::from(g) / 255.0, f32::from(b) / 255.0, 1.0]
        }
        Color::Indexed(idx) => xterm256_to_rgba(idx),
        Color::Named(named) => named_color_to_rgba(named),
    }
}

/// Named カラー → RGBA（Catppuccin Mocha 準拠）。
fn named_color_to_rgba(named: NamedColor) -> [f32; 4] {
    // Catppuccin Mocha カラーパレット（通常色と明色は同じ値）
    match named {
        NamedColor::Black => hex_rgba(0x45, 0x47, 0x5a), // Surface0
        NamedColor::Red | NamedColor::BrightRed => hex_rgba(0xf3, 0x8b, 0xa8), // Red
        NamedColor::Green | NamedColor::BrightGreen => hex_rgba(0xa6, 0xe3, 0xa1), // Green
        NamedColor::Yellow | NamedColor::BrightYellow => hex_rgba(0xf9, 0xe2, 0xaf), // Yellow
        NamedColor::Blue | NamedColor::BrightBlue => hex_rgba(0x89, 0xb4, 0xfa), // Blue
        NamedColor::Magenta | NamedColor::BrightMagenta => hex_rgba(0xcb, 0xa6, 0xf7), // Mauve
        NamedColor::Cyan | NamedColor::BrightCyan => hex_rgba(0x89, 0xdc, 0xeb), // Sky
        NamedColor::White => hex_rgba(0xba, 0xc2, 0xde), // Subtext1
        NamedColor::BrightBlack => hex_rgba(0x58, 0x5b, 0x70), // Surface2
        NamedColor::BrightWhite | NamedColor::Foreground => hex_rgba(0xcd, 0xd6, 0xf4), // Text
        NamedColor::Background => hex_rgba(0x1e, 0x1e, 0x2e), // Base
    }
}

/// RGB バイトから `[f32; 4]` に変換する。
fn hex_rgba(r: u8, g: u8, b: u8) -> [f32; 4] {
    [f32::from(r) / 255.0, f32::from(g) / 255.0, f32::from(b) / 255.0, 1.0]
}

/// xterm 256 色パレット → RGBA。
/// 簡易実装: 基本16色のみマッピング、残りはグレースケール近似。
fn xterm256_to_rgba(idx: u8) -> [f32; 4] {
    // 0-15: 基本色（named_color_to_rgba にマッピング）
    match idx {
        0 => named_color_to_rgba(NamedColor::Black),
        1 => named_color_to_rgba(NamedColor::Red),
        2 => named_color_to_rgba(NamedColor::Green),
        3 => named_color_to_rgba(NamedColor::Yellow),
        4 => named_color_to_rgba(NamedColor::Blue),
        5 => named_color_to_rgba(NamedColor::Magenta),
        6 => named_color_to_rgba(NamedColor::Cyan),
        7 => named_color_to_rgba(NamedColor::White),
        8 => named_color_to_rgba(NamedColor::BrightBlack),
        9 => named_color_to_rgba(NamedColor::BrightRed),
        10 => named_color_to_rgba(NamedColor::BrightGreen),
        11 => named_color_to_rgba(NamedColor::BrightYellow),
        12 => named_color_to_rgba(NamedColor::BrightBlue),
        13 => named_color_to_rgba(NamedColor::BrightMagenta),
        14 => named_color_to_rgba(NamedColor::BrightCyan),
        15 => named_color_to_rgba(NamedColor::BrightWhite),
        // 16-231: 6×6×6 カラーキューブ
        16..=231 => {
            let v = idx - 16;
            let b_idx = v % 6;
            let g_idx = (v / 6) % 6;
            let r_idx = v / 36;
            let to_f = |n: u8| if n == 0 { 0.0 } else { (55.0 + f32::from(n) * 40.0) / 255.0 };
            [to_f(r_idx), to_f(g_idx), to_f(b_idx), 1.0]
        }
        // 232-255: グレースケール
        232..=255 => {
            let v = (f32::from(idx - 232) * 10.0 + 8.0) / 255.0;
            [v, v, v, 1.0]
        }
    }
}
