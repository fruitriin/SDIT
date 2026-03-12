mod app;
mod config_watcher;
mod event_loop;
mod headless;
mod input;
mod render;
mod window;
mod window_ops;

use winit::event_loop::EventLoop;

use app::{SditApp, SditEvent};

fn main() {
    env_logger::init();

    if std::env::args().any(|a| a == "--headless") {
        log::info!("SDIT starting in headless mode");
        headless::run_headless();
    }

    let smoke_test =
        cfg!(debug_assertions) && std::env::var("SDIT_SMOKE_TEST").as_deref() == Ok("1");

    let config = sdit_core::config::Config::load(&sdit_core::config::Config::default_path());
    log::info!("SDIT starting (font: {} {}px)", config.font.family, config.font.size);
    let event_loop = EventLoop::<SditEvent>::with_user_event().build().unwrap();
    let proxy = event_loop.create_proxy();

    // 設定ファイル監視を開始（_watcher をドロップするとウォッチャーが停止するため保持する）
    let _watcher = config_watcher::spawn_config_watcher(
        &sdit_core::config::Config::default_path(),
        proxy.clone(),
    );

    let mut app = SditApp::new(proxy, smoke_test, &config);
    event_loop.run_app(&mut app).unwrap();
}
