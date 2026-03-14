//! wgpu パイプライン。
//!
//! - `GpuContext`: Surface / Device / Queue の初期化と管理
//! - `CellVertex`: 1セル分のインスタンスデータ
//! - `CellPipeline`: ターミナルグリッドをセル単位でレンダリングするパイプライン

use std::collections::HashSet;
use std::sync::Arc;

use crate::grid::Grid;
use crate::grid::{Cell, CellFlags, Color, Dimensions, NamedColor};
use crate::index::{Column, Line, Point};
use anyhow::{Context as _, Result};
use bytemuck::{Pod, Zeroable};
use winit::window::Window;

use super::atlas::Atlas;
use super::font::{FontContext, GlyphEntry, ShapedGlyph};

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
    /// `bg_pipeline` が `Some` の場合、クリアカラーの後に背景画像を描画してから
    /// `pipelines` 内の各パイプラインを順番に描画する。
    /// サイドバー + ターミナルの2パス描画に対応。
    pub fn render_frame(
        &self,
        pipelines: &[&CellPipeline],
        clear_color: [f32; 4],
        bg_pipeline: Option<&BackgroundPipeline>,
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

            // 背景画像をセルより前に描画する
            if let Some(bg) = bg_pipeline {
                pass.set_pipeline(&bg.pipeline);
                pass.set_bind_group(0, &bg.bind_group, &[]);
                pass.draw(0..6, 0..1);
            }

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
    /// カラーグリフフラグ。1.0 = カラー絵文字（fg 色を無視してテクスチャ色を使用）、0.0 = 通常グリフ。
    pub is_color_glyph: f32,
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
                wgpu::VertexAttribute { shader_location: 7, offset: 76, format: F::Float32 },
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
    /// 描画開始 X オフセット（ピクセル）。サイドバー + パディング分のオフセットに使用。
    origin_x: f32,
    /// 描画開始 Y オフセット（ピクセル）。パディング分のオフセットに使用。
    origin_y: f32,
    /// bytemuck の Pod/Zeroable を維持するためのパディング（16 バイトアライメント）。
    _pad: f32,
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
    #[allow(clippy::too_many_arguments)]
    pub fn update_uniforms(
        &self,
        queue: &wgpu::Queue,
        cell_size: [f32; 2],
        grid_size: [f32; 2],
        surface_size: [f32; 2],
        atlas_size: f32,
        origin_x: f32,
        origin_y: f32,
    ) {
        let uniforms = Uniforms {
            cell_size,
            grid_size,
            surface_size,
            atlas_size,
            origin_x,
            origin_y,
            _pad: 0.0,
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));
    }

    /// Grid からセルデータを構築して頂点バッファを更新する。
    ///
    /// `cursor_pos` はカーソル位置 `(column, row)` で、該当セルの fg/bg を反転描画する。
    /// `cursor_color` はカーソルの背景色。`Some` の場合は反転ではなくその色を使う。
    /// `selection` は選択範囲 `(start, end)` で、範囲内のセルの fg/bg を反転描画する。
    /// `url_hover` は `(row, start_col, end_col)` で、範囲内のセルを青色で描画する。
    /// `search_matches` は `(row, start_col, end_col)` のスライスで、マッチセルを黄色ハイライトする。
    /// `current_search_match` は現在フォーカス中のマッチで、オレンジ色でハイライトする。
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
        cursor_color: Option<[f32; 4]>,
        selection: Option<((usize, usize), (usize, usize))>,
        url_hover: Option<(usize, usize, usize)>,
        search_matches: Option<&[(usize, usize, usize)]>,
        current_search_match: Option<(usize, usize, usize)>,
        selection_fg: Option<[f32; 4]>,
        selection_bg: Option<[f32; 4]>,
        minimum_contrast: f32,
    ) {
        let rows = grid.screen_lines();
        let cols = grid.columns();

        // Uniforms を更新。origin_x/origin_y は呼び出し側で設定する場合もあるが、
        // update_from_grid では常に 0.0（サイドバーオフセットなし）を使用する。
        self.update_uniforms(
            queue,
            cell_size,
            [cols as f32, rows as f32],
            surface_size,
            atlas_size,
            0.0,
            0.0,
        );

        // 検索マッチセルの高速ルックアップ用 HashSet を構築（M-3: O(N×M) → O(1)）。
        let match_set: HashSet<(usize, usize)> = search_matches
            .map(|matches| {
                matches.iter().flat_map(|&(mr, ms, me)| (ms..me).map(move |c| (mr, c))).collect()
            })
            .unwrap_or_default();

        let mut vertices: Vec<CellVertex> = Vec::with_capacity(rows * cols);

        for row in 0..rows {
            // 行テキストを抽出（WIDE_CHAR_SPACER を除く）して shape_line() でシェーピング。
            let mut line_text = String::with_capacity(cols);
            for col in 0..cols {
                #[allow(clippy::cast_possible_wrap)]
                let point = Point::new(Line(row as i32), Column(col));
                let cell = &grid[point];
                if !cell.flags.contains(CellFlags::WIDE_CHAR_SPACER) {
                    line_text.push(cell.c);
                }
            }
            let shaped = font_ctx.shape_line(&line_text, atlas);

            // ShapedGlyph からカラム → グリフ情報のマップを構築。
            // col_glyph_map[col] = Some((entry, num_cells, is_first_cell))
            let col_glyph_map = build_col_glyph_map(&shaped, cols);

            #[allow(clippy::needless_range_loop)]
            for col in 0..cols {
                #[allow(clippy::cast_possible_wrap)]
                let point = Point::new(Line(row as i32), Column(col));
                let cell = &grid[point];

                // 色の決定（カーソル・選択・検索マッチ・URL ホバー）。
                let is_cursor = cursor_pos == Some((col, row));
                let is_selected = selection.is_some_and(|sel| is_in_selection(col, row, sel));
                let is_url_hovered =
                    url_hover.is_some_and(|(hr, hs, he)| row == hr && col >= hs && col < he);
                let is_current_match = current_search_match
                    .is_some_and(|(mr, ms, me)| row == mr && col >= ms && col < me);
                let is_search_match = match_set.contains(&(row, col));
                let (bg, fg) = if is_cursor {
                    // カーソル色が設定されていればそれを使用し、なければ反転
                    if let Some(c) = cursor_color {
                        (c, color_to_rgba(cell.bg))
                    } else {
                        (color_to_rgba(cell.fg), color_to_rgba(cell.bg))
                    }
                } else if is_selected {
                    // 選択色: 設定があればそれを使用し、なければ fg/bg 反転
                    let sel_bg = selection_bg.unwrap_or_else(|| color_to_rgba(cell.fg));
                    let sel_fg = selection_fg.unwrap_or_else(|| color_to_rgba(cell.bg));
                    (sel_bg, sel_fg)
                } else if is_current_match {
                    (hex_rgba(0xfa, 0xb3, 0x87), [0.0, 0.0, 0.0, 1.0])
                } else if is_search_match {
                    (hex_rgba(0xf9, 0xe2, 0xaf), [0.0, 0.0, 0.0, 1.0])
                } else if is_url_hovered {
                    (color_to_rgba(cell.bg), [0.4, 0.6, 1.0, 1.0])
                } else {
                    let raw_bg = color_to_rgba(cell.bg);
                    let raw_fg = color_to_rgba(cell.fg);
                    // minimum_contrast が有効な場合、fg 色を調整する
                    let adjusted_fg = if minimum_contrast > 1.0 {
                        use crate::config::color::apply_minimum_contrast;
                        let fg3 = [raw_fg[0], raw_fg[1], raw_fg[2]];
                        let bg3 = [raw_bg[0], raw_bg[1], raw_bg[2]];
                        let adj = apply_minimum_contrast(fg3, bg3, minimum_contrast);
                        [adj[0], adj[1], adj[2], raw_fg[3]]
                    } else {
                        raw_fg
                    };
                    (raw_bg, adjusted_fg)
                };

                // カラムマップからグリフ情報を取得。
                if let Some(ref info) = col_glyph_map[col] {
                    if info.is_first_cell {
                        // リガチャ/全角/通常グリフの最初のセル: グリフ情報あり。
                        let (uv, glyph_offset, glyph_size, is_color_glyph) =
                            glyph_entry_to_vertex_data(info.entry.as_ref(), atlas_size);
                        vertices.push(CellVertex {
                            bg,
                            fg,
                            grid_pos: [col as f32, row as f32],
                            uv,
                            glyph_offset,
                            glyph_size,
                            cell_width_scale: info.num_cells as f32,
                            is_color_glyph,
                        });
                    } else {
                        // リガチャ/全角の後続セル: 背景のみ描画。
                        vertices.push(CellVertex {
                            bg,
                            fg: [0.0; 4],
                            grid_pos: [col as f32, row as f32],
                            uv: [0.0; 4],
                            glyph_offset: [0.0; 2],
                            glyph_size: [0.0; 2],
                            cell_width_scale: 1.0,
                            is_color_glyph: 0.0,
                        });
                    }
                } else {
                    // マップにない（余剰カラム等）: 背景のみ。
                    vertices.push(CellVertex {
                        bg,
                        fg: [0.0; 4],
                        grid_pos: [col as f32, row as f32],
                        uv: [0.0; 4],
                        glyph_offset: [0.0; 2],
                        glyph_size: [0.0; 2],
                        cell_width_scale: 1.0,
                        is_color_glyph: 0.0,
                    });
                }
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
// リガチャ・カラムマップヘルパー
// ---------------------------------------------------------------------------

/// カラムごとのグリフ情報。
struct ColGlyphInfo {
    /// グリフエントリ（None = スペース等の描画不要グリフ）。
    entry: Option<GlyphEntry>,
    /// このグリフが占めるセル数。
    num_cells: usize,
    /// このカラムがグリフの最初のセルかどうか。
    is_first_cell: bool,
}

/// `ShapedGlyph` のリストからカラム → グリフ情報のマップを構築する。
///
/// 返り値は `cols` 長のベクタで、各要素は `Some(ColGlyphInfo)` または `None`。
fn build_col_glyph_map(shaped: &[ShapedGlyph], cols: usize) -> Vec<Option<ColGlyphInfo>> {
    let mut map: Vec<Option<ColGlyphInfo>> = (0..cols).map(|_| None).collect();

    for sg in shaped {
        if sg.start_col >= cols {
            continue;
        }
        // 最初のセル: グリフ情報あり。
        map[sg.start_col] = Some(ColGlyphInfo {
            entry: sg.entry.clone(),
            num_cells: sg.num_cells,
            is_first_cell: true,
        });
        // 後続セル: 背景のみ描画。
        for offset in 1..sg.num_cells {
            let c = sg.start_col + offset;
            if c < cols {
                map[c] = Some(ColGlyphInfo { entry: None, num_cells: 1, is_first_cell: false });
            }
        }
    }

    map
}

/// `GlyphEntry` から `CellVertex` のグリフデータ (uv, offset, size, `is_color`) を生成する。
fn glyph_entry_to_vertex_data(
    entry: Option<&GlyphEntry>,
    atlas_size: f32,
) -> ([f32; 4], [f32; 2], [f32; 2], f32) {
    if let Some(e) = entry {
        let r = e.region;
        let uv = [
            r.x as f32 / atlas_size,
            r.y as f32 / atlas_size,
            (r.x + r.width) as f32 / atlas_size,
            (r.y + r.height) as f32 / atlas_size,
        ];
        let offset = [e.placement_left as f32, e.placement_top as f32];
        let size = [r.width as f32, r.height as f32];
        let color_flag = if e.is_color { 1.0_f32 } else { 0.0_f32 };
        (uv, offset, size, color_flag)
    } else {
        ([0.0_f32; 4], [0.0_f32; 2], [0.0_f32; 2], 0.0_f32)
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

// ---------------------------------------------------------------------------
// BackgroundPipeline — 背景画像描画
// ---------------------------------------------------------------------------

/// 背景画像のユニフォームデータ。
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct BgUniforms {
    /// フィットモード: 0=contain, 1=cover, 2=fill
    fit_mode: u32,
    /// 不透明度
    opacity: f32,
    /// ウィンドウサイズ（ピクセル）
    surface_size: [f32; 2],
    /// 画像サイズ（ピクセル）
    image_size: [f32; 2],
    /// 16 バイトアライメント用パディング
    _pad: [f32; 2],
}

/// 背景画像をレンダリングするパイプライン。
pub struct BackgroundPipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    /// 画像サイズ（ピクセル）。ユニフォーム更新用。
    pub image_size: [f32; 2],
}

/// 背景画像パイプラインの入力パラメータを検証する。
///
/// バリデーション成功時は `bytes_per_row` を返す。失敗時は `None`。
/// wgpu device を必要とせずユニットテスト可能。
pub(crate) fn validate_background_image_params(
    image_data: &[u8],
    image_width: u32,
    image_height: u32,
) -> Option<u32> {
    const MAX_DIM: u32 = 4096;
    const MAX_PIXELS: u64 = 4096 * 4096;
    if image_width == 0 || image_height == 0 || image_width > MAX_DIM || image_height > MAX_DIM {
        return None;
    }
    let pixel_count = u64::from(image_width) * u64::from(image_height);
    if pixel_count > MAX_PIXELS {
        return None;
    }
    let expected_len = (pixel_count * 4) as usize;
    if image_data.len() != expected_len {
        return None;
    }
    image_width.checked_mul(4)
}

impl BackgroundPipeline {
    /// 背景画像テクスチャから `BackgroundPipeline` を作成する。
    ///
    /// `image_data`: RGBA8 ピクセルデータ（行優先）
    /// `image_width` / `image_height`: 画像サイズ（ピクセル）
    /// `fit_mode`: 0=contain, 1=cover, 2=fill
    /// `opacity`: 不透明度 (0.0-1.0)
    ///
    /// バリデーション失敗時は `None` を返す:
    /// - `image_width` または `image_height` が 0 または 4096 超
    /// - ピクセル数が 16,777,216 (4096×4096) 超
    /// - `image_data` の長さが `width * height * 4` と一致しない
    /// - `bytes_per_row` の計算がオーバーフローする場合
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
        image_data: &[u8],
        image_width: u32,
        image_height: u32,
        fit_mode: u32,
        opacity: f32,
        surface_size: [f32; 2],
    ) -> Option<Self> {
        // バリデーション（寸法・ピクセル数・データ長・bytes_per_row）
        let bytes_per_row = validate_background_image_params(image_data, image_width, image_height)
            .unwrap_or_else(|| {
                log::warn!(
                    "BackgroundPipeline::new: validation failed for image {}x{}",
                    image_width,
                    image_height
                );
                0
            });
        if bytes_per_row == 0 {
            return None;
        }

        // テクスチャ作成
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bg_image_texture"),
            size: wgpu::Extent3d {
                width: image_width,
                height: image_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            image_data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(image_height),
            },
            wgpu::Extent3d { width: image_width, height: image_height, depth_or_array_layers: 1 },
        );
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // サンプラー
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("bg_image_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // ユニフォームバッファ
        let uniforms = BgUniforms {
            fit_mode,
            opacity,
            surface_size,
            image_size: [image_width as f32, image_height as f32],
            _pad: [0.0; 2],
        };
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bg_uniform_buffer"),
            size: std::mem::size_of::<BgUniforms>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

        // バインドグループレイアウト
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bg_bgl"),
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

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bg_bind_group"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: uniform_buffer.as_entire_binding() },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        // シェーダー
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bg_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/background.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("bg_pipeline_layout"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("bg_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[], // 頂点バッファなし（vertex_index で生成）
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

        Some(Self {
            pipeline,
            bind_group,
            uniform_buffer,
            image_size: [image_width as f32, image_height as f32],
        })
    }

    /// サーフェスサイズが変わったときにユニフォームを更新する。
    pub fn update_surface_size(
        &self,
        queue: &wgpu::Queue,
        surface_size: [f32; 2],
        fit_mode: u32,
        opacity: f32,
    ) {
        let uniforms = BgUniforms {
            fit_mode,
            opacity,
            surface_size,
            image_size: self.image_size,
            _pad: [0.0; 2],
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));
    }
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

// ---------------------------------------------------------------------------
// テスト
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- H-1: BackgroundPipeline バリデーション関数テスト ---

    #[test]
    fn validate_bg_image_zero_width_returns_none() {
        let data = vec![0u8; 4]; // 1x1 の RGBA データだが width=0 に設定
        assert!(validate_background_image_params(&data, 0, 1).is_none(), "width=0 は None を返す");
    }

    #[test]
    fn validate_bg_image_zero_height_returns_none() {
        let data = vec![0u8; 4];
        assert!(validate_background_image_params(&data, 1, 0).is_none(), "height=0 は None を返す");
    }

    #[test]
    fn validate_bg_image_too_large_dimension_returns_none() {
        // 4097x1 は MAX_DIM(4096) 超
        let data = vec![0u8; 4097 * 1 * 4];
        assert!(
            validate_background_image_params(&data, 4097, 1).is_none(),
            "width=4097 は None を返す"
        );
    }

    #[test]
    fn validate_bg_image_max_allowed_dimension() {
        // 4096x1 は許容範囲内
        let data = vec![0u8; 4096 * 1 * 4];
        assert!(
            validate_background_image_params(&data, 4096, 1).is_some(),
            "width=4096 は許容される"
        );
    }

    #[test]
    fn validate_bg_image_data_length_mismatch_returns_none() {
        // 2x2 には 16 bytes 必要だが、8 bytes しか渡さない
        let data = vec![0u8; 8];
        assert!(
            validate_background_image_params(&data, 2, 2).is_none(),
            "データ長不一致は None を返す"
        );
    }

    #[test]
    fn validate_bg_image_valid_params() {
        // 正常な 2x2 RGBA8 画像
        let data = vec![0u8; 2 * 2 * 4];
        let result = validate_background_image_params(&data, 2, 2);
        assert!(result.is_some(), "正常パラメータは Some を返す");
        assert_eq!(result.unwrap(), 8, "bytes_per_row = width * 4 = 8");
    }

    #[test]
    fn validate_bg_image_1x1() {
        // 1x1 の最小ケース
        let data = vec![255u8, 0, 0, 255]; // 赤い 1 ピクセル
        let result = validate_background_image_params(&data, 1, 1);
        assert!(result.is_some(), "1x1 は許容される");
        assert_eq!(result.unwrap(), 4, "bytes_per_row = 1 * 4 = 4");
    }
}
