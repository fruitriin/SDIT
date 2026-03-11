use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

use sdit_core::grid::{Cell, Color, Dimensions, Grid, NamedColor};
use sdit_core::index::{Column, Line, Point};
use sdit_render::atlas::Atlas;
use sdit_render::font::FontContext;
use sdit_render::pipeline::{CellPipeline, GpuContext};

/// カスタムイベント型（後の Phase で拡張する）
#[derive(Debug)]
pub enum SditEvent {
    Redraw,
}

struct SditApp {
    window: Option<Arc<Window>>,
    gpu: Option<GpuContext<'static>>,
    cell_pipeline: Option<CellPipeline>,
    atlas: Option<Atlas>,
    font_ctx: Option<FontContext>,
    grid: Option<Grid<Cell>>,
}

impl SditApp {
    fn new() -> Self {
        Self {
            window: None,
            gpu: None,
            cell_pipeline: None,
            atlas: None,
            font_ctx: None,
            grid: None,
        }
    }

    /// Grid・アトラス・フォントコンテキスト・パイプラインを初期化する。
    fn init_render(&mut self) {
        let Some(gpu) = &self.gpu else { return };

        // 24×80 の Grid を作成して "Hello, SDIT!" を書き込む。
        let mut grid = Grid::<Cell>::new(24, 80, 1000);
        write_str_to_grid(&mut grid, 0, 0, "Hello, SDIT!");

        // フォントコンテキストを作成。
        let mut font_ctx = FontContext::new(14.0, 1.2);
        let metrics = *font_ctx.metrics();

        // テクスチャアトラスを作成。
        let mut atlas = Atlas::new(&gpu.device, 512);

        let rows = grid.screen_lines();
        let cols = grid.columns();
        let cell_size = [metrics.cell_width, metrics.cell_height];
        let surface_size = [gpu.surface_config.width as f32, gpu.surface_config.height as f32];

        // CellPipeline を作成。
        let mut cell_pipeline = CellPipeline::new(&gpu.device, gpu.surface_config.format, &atlas);

        // Grid からセルデータを GPU に転送。
        let atlas_size_f32 = atlas.size() as f32;
        cell_pipeline.update_from_grid(
            &gpu.queue,
            &grid,
            &mut font_ctx,
            &mut atlas,
            atlas_size_f32,
            cell_size,
            surface_size,
        );

        // アトラスを GPU にアップロード。
        atlas.upload_if_dirty(&gpu.queue);

        // Uniforms を更新（bind_group は atlas の texture_view を参照しているため、
        // アトラスを先にアップロードした後に uniform だけ更新）。
        cell_pipeline.update_uniforms(
            &gpu.queue,
            cell_size,
            [cols as f32, rows as f32],
            surface_size,
            atlas_size_f32,
        );

        self.grid = Some(grid);
        self.font_ctx = Some(font_ctx);
        self.atlas = Some(atlas);
        self.cell_pipeline = Some(cell_pipeline);
    }
}

impl ApplicationHandler<SditEvent> for SditApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("SDIT")
            .with_inner_size(winit::dpi::LogicalSize::new(800.0_f64, 600.0_f64));

        let window = Arc::new(event_loop.create_window(attrs).unwrap());
        let gpu = GpuContext::new(&window).unwrap();

        self.window = Some(window);
        self.gpu = Some(gpu);

        self.init_render();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(gpu) = &mut self.gpu {
                    gpu.resize(size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(gpu) = &self.gpu {
                    match gpu.render_frame(self.cell_pipeline.as_ref()) {
                        Ok(()) => {}
                        Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                            if let Some(gpu) = &mut self.gpu {
                                let (w, h) = (gpu.surface_config.width, gpu.surface_config.height);
                                gpu.resize(w, h);
                            }
                        }
                        Err(e) => log::error!("Render error: {e}"),
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

/// Grid の (row, col) から1行の文字列を書き込む。
fn write_str_to_grid(grid: &mut Grid<Cell>, row: usize, start_col: usize, s: &str) {
    for (i, c) in s.chars().enumerate() {
        let col = start_col + i;
        if col >= grid.columns() {
            break;
        }
        // row は grid.screen_lines() 内で i32 に収まる。
        #[allow(clippy::cast_possible_wrap)]
        let point = Point::new(Line(row as i32), Column(col));
        grid[point] = Cell {
            c,
            fg: Color::Named(NamedColor::BrightWhite),
            bg: Color::Named(NamedColor::Background),
            ..Cell::default()
        };
    }
}

fn main() {
    env_logger::init();
    log::info!("SDIT starting");
    let event_loop = EventLoop::<SditEvent>::with_user_event().build().unwrap();
    let mut app = SditApp::new();
    event_loop.run_app(&mut app).unwrap();
}
