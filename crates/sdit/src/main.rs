use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

use sdit_render::pipeline::GpuContext;

/// カスタムイベント型（後の Phase で拡張する）
#[derive(Debug)]
pub enum SditEvent {
    Redraw,
}

struct SditApp {
    window: Option<Arc<Window>>,
    gpu: Option<GpuContext<'static>>,
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
                    match gpu.render_frame() {
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

fn main() {
    env_logger::init();
    log::info!("SDIT starting");
    let event_loop = EventLoop::<SditEvent>::with_user_event().build().unwrap();
    let mut app = SditApp { window: None, gpu: None };
    event_loop.run_app(&mut app).unwrap();
}
