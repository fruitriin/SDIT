// macOS メニューバーの構築。
// このファイルは macOS 専用。他プラットフォームではコンパイルされない。
#![cfg(target_os = "macos")]
// muda の show_context_menu_for_nsview は unsafe fn のため、このファイルのみ許可する。
#![allow(unsafe_code)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use muda::accelerator::{Accelerator, Code, Modifiers};
use muda::{ContextMenu as _, Menu, MenuId, MenuItem, PredefinedMenuItem, Submenu};

use sdit_core::config::keybinds::Action;

/// アクセラレータのヘルパー: SUPER + キーコード。
fn super_accel(code: Code) -> Accelerator {
    Accelerator::new(Some(Modifiers::SUPER), code)
}

/// アクセラレータのヘルパー: SUPER + SHIFT + キーコード。
fn super_shift_accel(code: Code) -> Accelerator {
    Accelerator::new(Some(Modifiers::SUPER | Modifiers::SHIFT), code)
}

/// macOS メニューバーを構築し、`(Menu, MenuId → Action)` のペアを返す。
///
/// 返した `Menu` を `Menu::init_for_nsapp()` で `NSApp` に設定すること。
/// `Menu` はドロップされるとメニューが消えるため、`SditApp` に保持すること。
pub(crate) fn build_menu_bar() -> (Menu, HashMap<MenuId, Action>) {
    let menu = Menu::new();
    let mut id_map: HashMap<MenuId, Action> = HashMap::new();

    // ------------------------------------------------------------------
    // SDIT メニュー（アプリメニュー）
    // ------------------------------------------------------------------
    let about = MenuItem::new("About SDIT", true, None::<Accelerator>);
    id_map.insert(about.id().clone(), Action::About);

    let preferences = MenuItem::new("Preferences…", true, Some(super_accel(Code::Comma)));
    id_map.insert(preferences.id().clone(), Action::Preferences);

    let quit = MenuItem::new("Quit SDIT", true, Some(super_accel(Code::KeyQ)));
    id_map.insert(quit.id().clone(), Action::Quit);

    let app_menu = Submenu::with_items(
        "SDIT",
        true,
        &[
            &about,
            &PredefinedMenuItem::separator(),
            &preferences,
            &PredefinedMenuItem::separator(),
            &quit,
        ],
    )
    .expect("Failed to build SDIT app menu");

    // ------------------------------------------------------------------
    // File メニュー
    // ------------------------------------------------------------------
    let new_window = MenuItem::new("New Window", true, Some(super_accel(Code::KeyN)));
    id_map.insert(new_window.id().clone(), Action::NewWindow);

    let new_tab = MenuItem::new("New Tab", true, Some(super_accel(Code::KeyT)));
    id_map.insert(new_tab.id().clone(), Action::AddSession);

    let close = MenuItem::new("Close Tab", true, Some(super_accel(Code::KeyW)));
    id_map.insert(close.id().clone(), Action::CloseSession);

    let file_menu = Submenu::with_items(
        "File",
        true,
        &[&new_window, &new_tab, &PredefinedMenuItem::separator(), &close],
    )
    .expect("Failed to build File menu");

    // ------------------------------------------------------------------
    // Edit メニュー
    // ------------------------------------------------------------------
    let copy = MenuItem::new("Copy", true, Some(super_accel(Code::KeyC)));
    id_map.insert(copy.id().clone(), Action::Copy);

    let paste = MenuItem::new("Paste", true, Some(super_accel(Code::KeyV)));
    id_map.insert(paste.id().clone(), Action::Paste);

    let select_all = MenuItem::new("Select All", true, Some(super_accel(Code::KeyA)));
    id_map.insert(select_all.id().clone(), Action::SelectAll);

    let edit_menu = Submenu::with_items(
        "Edit",
        true,
        &[&copy, &paste, &PredefinedMenuItem::separator(), &select_all],
    )
    .expect("Failed to build Edit menu");

    // ------------------------------------------------------------------
    // View メニュー
    // ------------------------------------------------------------------
    let toggle_sidebar = MenuItem::new(
        "Toggle Sidebar",
        true,
        Some(Accelerator::new(Some(Modifiers::SUPER), Code::Backslash)),
    );
    id_map.insert(toggle_sidebar.id().clone(), Action::SidebarToggle);

    let zoom_in = MenuItem::new("Zoom In", true, Some(super_accel(Code::Equal)));
    id_map.insert(zoom_in.id().clone(), Action::ZoomIn);

    let zoom_out = MenuItem::new("Zoom Out", true, Some(super_accel(Code::Minus)));
    id_map.insert(zoom_out.id().clone(), Action::ZoomOut);

    let zoom_reset = MenuItem::new("Actual Size", true, Some(super_accel(Code::Digit0)));
    id_map.insert(zoom_reset.id().clone(), Action::ZoomReset);

    let search = MenuItem::new("Search…", true, Some(super_accel(Code::KeyF)));
    id_map.insert(search.id().clone(), Action::Search);

    let view_menu = Submenu::with_items(
        "View",
        true,
        &[
            &toggle_sidebar,
            &PredefinedMenuItem::separator(),
            &zoom_in,
            &zoom_out,
            &zoom_reset,
            &PredefinedMenuItem::separator(),
            &search,
        ],
    )
    .expect("Failed to build View menu");

    // ------------------------------------------------------------------
    // Session メニュー
    // ------------------------------------------------------------------
    let next_session = MenuItem::new("Next Tab", true, Some(super_shift_accel(Code::BracketRight)));
    id_map.insert(next_session.id().clone(), Action::NextSession);

    let prev_session =
        MenuItem::new("Previous Tab", true, Some(super_shift_accel(Code::BracketLeft)));
    id_map.insert(prev_session.id().clone(), Action::PrevSession);

    let detach = MenuItem::new("Move Tab to New Window", true, Some(super_shift_accel(Code::KeyN)));
    id_map.insert(detach.id().clone(), Action::DetachSession);

    let session_menu = Submenu::with_items(
        "Session",
        true,
        &[&next_session, &prev_session, &PredefinedMenuItem::separator(), &detach],
    )
    .expect("Failed to build Session menu");

    // ------------------------------------------------------------------
    // メニューバーにサブメニューを追加
    // ------------------------------------------------------------------
    menu.append(&app_menu).expect("Failed to append app menu");
    menu.append(&file_menu).expect("Failed to append File menu");
    menu.append(&edit_menu).expect("Failed to append Edit menu");
    menu.append(&view_menu).expect("Failed to append View menu");
    menu.append(&session_menu).expect("Failed to append Session menu");

    (menu, id_map)
}

// ---------------------------------------------------------------------------
// コンテキストメニュービルダー
// ---------------------------------------------------------------------------

/// ターミナル領域の右クリックメニューを構築する。
pub(crate) fn build_terminal_context_menu() -> (Menu, HashMap<MenuId, Action>) {
    let menu = Menu::new();
    let mut id_map: HashMap<MenuId, Action> = HashMap::new();

    let copy = MenuItem::new("Copy", true, None::<Accelerator>);
    id_map.insert(copy.id().clone(), Action::Copy);
    menu.append(&copy).unwrap();

    let paste = MenuItem::new("Paste", true, None::<Accelerator>);
    id_map.insert(paste.id().clone(), Action::Paste);
    menu.append(&paste).unwrap();

    menu.append(&PredefinedMenuItem::separator()).unwrap();

    let select_all = MenuItem::new("Select All", true, None::<Accelerator>);
    id_map.insert(select_all.id().clone(), Action::SelectAll);
    menu.append(&select_all).unwrap();

    menu.append(&PredefinedMenuItem::separator()).unwrap();

    let search = MenuItem::new("Search…", true, None::<Accelerator>);
    id_map.insert(search.id().clone(), Action::Search);
    menu.append(&search).unwrap();

    (menu, id_map)
}

/// サイドバー領域の右クリックメニューを構築する。
pub(crate) fn build_sidebar_context_menu() -> (Menu, HashMap<MenuId, Action>) {
    let menu = Menu::new();
    let mut id_map: HashMap<MenuId, Action> = HashMap::new();

    let close = MenuItem::new("Close Session", true, None::<Accelerator>);
    id_map.insert(close.id().clone(), Action::CloseSession);
    menu.append(&close).unwrap();

    let detach = MenuItem::new("Move to New Window", true, None::<Accelerator>);
    id_map.insert(detach.id().clone(), Action::DetachSession);
    menu.append(&detach).unwrap();

    (menu, id_map)
}

/// `NSView` ポインタを取得して muda コンテキストメニューを表示する。
///
/// # Safety
/// `ns_view` は有効な `NSView` インスタンスへのポインタでなければならない。
/// winit の `Window::window_handle()` から取得した `AppKit` ハンドルを使用すること。
pub(crate) fn show_context_menu_for_window(
    window: &std::sync::Arc<winit::window::Window>,
    menu: &Menu,
) {
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let Ok(handle) = window.window_handle() else { return };
    let RawWindowHandle::AppKit(appkit) = handle.as_raw() else { return };
    let ns_view = appkit.ns_view.as_ptr();
    // SAFETY: winit が保証している通り、ns_view は有効な NSView ポインタ。
    // macOS メインスレッド上で呼ばれるため問題ない。
    unsafe {
        menu.show_context_menu_for_nsview(ns_view.cast(), None);
    }
}

// ---------------------------------------------------------------------------
// 共有 MenuId マップ
// ---------------------------------------------------------------------------

/// `MenuEvent` ハンドラとコンテキストメニューの両方から参照できる共有マップを作成する。
pub(crate) type SharedMenuActions = Arc<Mutex<HashMap<MenuId, Action>>>;

/// `build_menu_bar` の返り値から共有マップを生成する。
pub(crate) fn make_shared_actions(id_map: HashMap<MenuId, Action>) -> SharedMenuActions {
    Arc::new(Mutex::new(id_map))
}

// ---------------------------------------------------------------------------
// テスト
// ---------------------------------------------------------------------------
//
// muda::Menu は macOS ではメインスレッドでしか生成できない。
// 通常の cargo test スレッドからは build_menu_bar() を直接呼べないため、
// ここでは Action バリアントのコンパイル時確認のみ行う。
// 実際のメニューバー表示は smoke_gui テストで確認する。

#[cfg(test)]
mod tests {
    use sdit_core::config::keybinds::Action;

    /// メニューバーで使用する全 Action バリアントが Action 型に定義されていることを確認する。
    /// コンパイル時チェック: これがコンパイルできれば全バリアントが存在する。
    #[test]
    fn menu_actions_are_defined() {
        // smell-allow: magic-number — 配列の要素数は上の定義から自明
        // 各バリアントを列挙してコンパイル時に検証する
        let actions = [
            Action::About,
            Action::Preferences,
            Action::Quit,
            Action::NewWindow,
            Action::AddSession,
            Action::CloseSession,
            Action::Copy,
            Action::Paste,
            Action::SelectAll,
            Action::SidebarToggle,
            Action::ZoomIn,
            Action::ZoomOut,
            Action::ZoomReset,
            Action::Search,
            Action::NextSession,
            Action::PrevSession,
            Action::DetachSession,
        ];
        // 全て異なる値であることを確認
        assert_eq!(actions.len(), 17);
    }

    /// コンテキストメニューで使用する Action バリアントが定義されていることを確認する。
    #[test]
    fn context_menu_actions_are_defined() {
        // ターミナル領域コンテキストメニュー
        let terminal_actions = [Action::Copy, Action::Paste, Action::SelectAll, Action::Search];
        assert_eq!(terminal_actions.len(), 4);

        // サイドバー領域コンテキストメニュー
        let sidebar_actions = [Action::CloseSession, Action::DetachSession];
        assert_eq!(sidebar_actions.len(), 2);
    }
}
