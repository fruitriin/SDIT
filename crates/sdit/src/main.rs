mod action_handlers;
mod app;
mod command_palette;
mod config_watcher;
mod confirm_close;
mod cwd_utils;
mod event_loop;
mod headless;
mod input;
#[cfg(target_os = "macos")]
mod menu;
mod quick_select;
mod quick_terminal;
mod rename;
mod render;
mod scrollbar;
mod search;
mod secure_input;
mod selection_utils;
mod url_hover;
mod vi_mode;
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

    // macOS メニューバーの初期化
    // _menu_bar をドロップするとメニューが消えるため変数に保持する。
    #[cfg(target_os = "macos")]
    let (_menu_bar, menu_actions) = {
        let (menu_bar, id_map) = menu::build_menu_bar();
        menu_bar.init_for_nsapp();

        let shared = menu::make_shared_actions(id_map);
        let handler_actions = shared.clone();
        let menu_proxy = proxy.clone();
        muda::MenuEvent::set_event_handler(Some(move |event: muda::MenuEvent| {
            if let Some(&action) = handler_actions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .get(event.id())
            {
                let _ = menu_proxy.send_event(SditEvent::MenuAction(action));
            }
        }));
        (menu_bar, shared)
    };

    #[cfg(target_os = "macos")]
    let mut app = SditApp::new(proxy, smoke_test, &config, menu_actions);
    #[cfg(not(target_os = "macos"))]
    let mut app = SditApp::new(proxy, smoke_test, &config);
    event_loop.run_app(&mut app).unwrap();
}
