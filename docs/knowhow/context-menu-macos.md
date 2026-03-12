# macOS コンテキストメニュー実装知見（Phase 11.2）

## 実装パターン

### muda による NSView ベースのコンテキストメニュー

macOS では `muda::ContextMenu::show_context_menu_for_nsview()` を使う。
この関数は `unsafe fn` のため、呼び出すファイルに `#![allow(unsafe_code)]` が必要。

```rust
// menu.rs の先頭
#![allow(unsafe_code)]

pub(crate) fn show_context_menu_for_window(window: &Arc<winit::window::Window>, menu: &Menu) {
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let Ok(handle) = window.window_handle() else { return };
    let RawWindowHandle::AppKit(appkit) = handle.as_raw() else { return };
    let ns_view = appkit.ns_view.as_ptr();
    unsafe {
        menu.show_context_menu_for_nsview(ns_view.cast(), None);
    }
}
```

### MenuEvent ハンドラとの共存

`MenuEvent::set_event_handler` はグローバルで1つしか登録できないため、
メニューバーのハンドラ登録後にコンテキストメニューの ID も同じ shared map に `extend()` する。

```
Arc<Mutex<HashMap<MenuId, Action>>>  // 共有アクションマップ
  ├── メニューバーの id_map (main.rs で初期化)
  └── コンテキストメニューの id_map (右クリック時か初期化時に extend)
```

- `extend()` 方式は ID が累積するが、Action バリアントの数は有限（~10件）なので許容範囲

### 右クリック判定とサイドバー領域の分岐

```rust
WindowEvent::MouseInput {
    state: ElementState::Pressed,
    button: MouseButton::Right,
    ..
} => {
    if let Some((x, y)) = self.cursor_position {
        let sidebar_width = metrics.cell_width * sidebar_cols as f32;
        if x < sidebar_width {
            // サイドバー領域 → サイドバーメニュー
        } else {
            // ターミナル領域 → ターミナルメニュー
        }
    }
}
```

## 注意点

### muda::Menu の初期化スレッド制約

`muda::Menu` は macOS ではメインスレッドでしか生成できない。
`cargo test` のテストスレッドからは `build_menu_bar()` 等を直接呼べないため、
ユニットテストでは Action バリアントのコンパイル時確認のみ行う。
GUI 動作の確認は `smoke_gui` テストで行う。

### dead_code 警告の扱い

`SditApp` に `terminal_ctx_menu` と `sidebar_ctx_menu` を保持する設計の場合、
`event_loop.rs` で毎回 `build_terminal_context_menu()` を呼ぶと、
フィールドが使われない（dead_code 警告）になる。

設計選択肢:
1. フィールドに保持 → 起動時1回構築、右クリック時に再利用（推奨）
2. 毎回構築 → フィールド不要、シンプルだがメモリアロケーションが都度発生

現状は2の実装になっているが、フィールド定義だけ1の設計になっているため警告が出る。
将来的にフィールドを使って再利用する方向への整合が必要。

### unsafe スコープの限定

CLAUDE.md 方針: `unsafe_code = "deny"` をワークスペース全体に適用。
`menu.rs` ファイルのみ `#![allow(unsafe_code)]` でスコープを限定することで、
他ファイルへの unsafe 汚染を防ぐ。
