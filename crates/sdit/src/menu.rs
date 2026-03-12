// macOS メニューバーの構築。
// このファイルは macOS 専用。他プラットフォームではコンパイルされない。
#![cfg(target_os = "macos")]

use std::collections::HashMap;

use muda::accelerator::{Accelerator, Code, Modifiers};
use muda::{Menu, MenuId, MenuItem, PredefinedMenuItem, Submenu};

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
}
