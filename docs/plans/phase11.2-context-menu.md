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
- `crates/sdit/src/app.rs`: `SditApp` に `menu_actions`, `terminal_ctx_menu`, `sidebar_ctx_menu` フィールドを追加（`#[cfg(target_os = "macos")]`）。コンテキストメニューは初期化時に1回構築して再利用。
- `crates/sdit/src/main.rs`: `make_shared_actions()` で共有マップを生成し `MenuEvent` ハンドラと `SditApp` で共有する構成に変更。
- `crates/sdit/src/event_loop.rs`: `WindowEvent::MouseInput { MouseButton::Right }` ハンドラを追加（`#[cfg(target_os = "macos")]`）。

### アーキテクチャ判断

- `MenuEvent::set_event_handler` はグローバルで1つしかないため、メニューバー用 id_map とコンテキストメニュー用 id_map を `Arc<Mutex<HashMap>>` で統一した。
- コンテキストメニューは `SditApp` 初期化時に1回だけ構築し、id_map を共有マップに追加。右クリック時は保持済みメニューを表示するだけ（M-1 修正）。
- `unsafe_code = "deny"` の制約に対し、`menu.rs` ファイルのみ `#![allow(unsafe_code)]` でスコープを限定。

## セキュリティレビュー結果

| 重要度 | ID | 内容 | 対応 |
|---|---|---|---|
| Medium | M-1 | `extend()` による MenuId の無制限蓄積 | **修正済み**: コンテキストメニューを初期化時に1回構築して再利用 |
| Low | L-1 | `menu.append().unwrap()` の一貫性不足 | `.expect()` への統一は将来対応 |
| Low | L-2 | `hit_test` の境界チェックの微妙な不整合 | `hit_test` は `session_count` 未満を保証。実害なし |
| Info | I-1 | `#![allow(unsafe_code)]` のファイル単位適用 | unsafe は1箇所のみ。スコープ限定済み |
| Info | I-2 | `PoisonError::into_inner` の使用 | HashMap 操作は軽量で整合性リスクは低い |
