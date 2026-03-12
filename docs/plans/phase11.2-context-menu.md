# Phase 11.2: 右クリックコンテキストメニュー

**概要**: ターミナル領域とサイドバー領域で右クリックメニューを表示する。

**状態: 完了**

## タスク

| タスク | 詳細 | 変更先クレート | 状態 |
|---|---|---|---|
| MouseButton::Right イベント処理 | 右クリック検出（現在は Left のみ実装） | sdit (`event_loop.rs`) | 完了 |
| ターミナル領域メニュー | Copy, Paste, Select All, Search | sdit (`menu.rs`) | 完了 |
| サイドバー領域メニュー | Close Session, Move to New Window | sdit (`menu.rs`) | 完了 |
| muda PopupMenu 統合 | ネイティブコンテキストメニューを muda で表示 | sdit | 完了 |

## 依存関係

- Phase 11.1（muda クレートの導入）
- Phase 6.2（Copy/Paste の実装）

## 実装詳細

### 変更ファイル

- `crates/sdit/src/menu.rs`: `build_terminal_context_menu()`, `build_sidebar_context_menu()`, `show_context_menu_for_window()`, `make_shared_actions()` を追加。ファイル先頭に `#![allow(unsafe_code)]` を追加（`show_context_menu_for_nsview` が `unsafe fn` のため）。
- `crates/sdit/src/app.rs`: `SditApp` に `menu_actions: SharedMenuActions` フィールドを追加（`#[cfg(target_os = "macos")]`）。`new()` に対応する引数を追加。
- `crates/sdit/src/main.rs`: `make_shared_actions()` で共有マップを生成し `MenuEvent` ハンドラと `SditApp` で共有する構成に変更。
- `crates/sdit/src/event_loop.rs`: `WindowEvent::MouseInput { MouseButton::Right }` ハンドラを追加（`#[cfg(target_os = "macos")]`）。

### アーキテクチャ判断

- `MenuEvent::set_event_handler` はグローバルで1つしかないため、メニューバー用 id_map とコンテキストメニュー用 id_map を `Arc<Mutex<HashMap>>` で統一した。
- 右クリック時にコンテキストメニューの id_map を共有マップに `extend()` することで、既存のメニューバーハンドラに変更を加えずに対応できる。
- `unsafe_code = "deny"` の制約に対し、`menu.rs` ファイルのみ `#![allow(unsafe_code)]` でスコープを限定。

## セキュリティレビュー結果

### Low

- **L-1**: コンテキストメニュー id_map の `extend()` は累積する（古いコンテキストメニューの ID が残り続ける）。悪用面は限定的（同じ Action バリアントのみ追加）だが、メモリ消費は微増する。現時点では許容範囲。将来的に大量のメニュー追加がある場合はクリーンアップを検討する。
