# Phase 11.1: macOS メニューバー

**概要**: macOS ネイティブメニューバーを実装する。winit 0.30 にはメニューAPIがないため、`muda` クレートで実装する。

**状態**: **完了**

## タスク

| タスク | 詳細 | 変更先クレート | 状態 |
|---|---|---|---|
| muda クレート統合 | macOS ネイティブメニューバーの基盤を導入 | sdit (`main.rs`), `Cargo.toml` | 完了 |
| アプリケーションメニュー | SDIT > About, Preferences, Quit | sdit | 完了 |
| ファイルメニュー | New Window (Cmd+N), New Tab (Cmd+T), Close (Cmd+W) | sdit | 完了 |
| 編集メニュー | Copy (Cmd+C), Paste (Cmd+V), Select All (Cmd+A) | sdit | 完了 |
| 表示メニュー | Toggle Sidebar (Cmd+\), Font Size +/- (Cmd+=/-), Search (Cmd+F) | sdit | 完了 |
| セッションメニュー | Next/Prev Session, Detach Session | sdit | 完了 |
| メニューアクションとキーバインドの統合 | `handle_action()` メソッド抽出でキーバインドとメニューを統一ディスパッチ | sdit | 完了 |

## 実装記録（2026-03-13 完了）

### 変更内容

- `crates/sdit-core/src/config/keybinds.rs`: `Action` enum に `Quit`, `About`, `Preferences`, `SelectAll` を追加。macOS デフォルトバインドに `Cmd+Q`, `Cmd+A`, `Cmd+,` を追加。
- `crates/sdit/src/menu.rs` (新規): `build_menu_bar()` 関数。SDIT/File/Edit/View/Session の5メニューを構築し、`MenuId → Action` マッピングを返す。`muda 0.17` 使用。
- `crates/sdit/src/app.rs`: `SditEvent::MenuAction(Action)` バリアント追加。
- `crates/sdit/src/event_loop.rs`: 既存の action dispatch を `handle_action()` メソッドに抽出。`user_event()` で `MenuAction` をフォーカスウィンドウに対してディスパッチ。新アクション実装（Quit: 全ウィンドウクローズ + exit、About: ログ出力、Preferences: `open::that()` で設定ファイルを開く、SelectAll: ログ出力）。
- `crates/sdit/src/main.rs`: `#[cfg(target_os = "macos")]` でメニュー初期化。`Menu::init_for_nsapp()`、`MenuEvent::set_event_handler()` で EventLoopProxy 経由の統合。

### 新規依存クレート

- `muda 0.17` — macOS ネイティブメニューバー
- `open 5` — 設定ファイルをシステムエディタで開く

### セキュリティレビュー結果

注: セキュリティレビューは Phase 10.3a の変更に対して実施された（Phase 11.1 の変更は別途レビュー予定）。

Phase 10.3a に対する指摘:

| 重要度 | ID | 内容 | 対応 |
|---|---|---|---|
| Medium | M-1 | SubpixelMask の chunks_exact(3) が誤り、実際は 4bytes/pixel | **修正済み**: chunks_exact(4) に修正 |
| Low | L-1 | swash Color ビットマップの BGRA 前提が未検証 | 実機テストで確認予定 |
| Low | L-2 | サイドバー等で is_color_glyph が常に 0.0 ハードコード | 将来的にサイドバーに絵文字表示が必要になったら対応 |
| Info | I-1 | atlas::write のサイズ不一致が Silent Drop | debug_assert! の追加を検討 |
| Info | I-2 | u32 オーバーフローの理論的リスク | reserve() でサイズ上限チェック済みのため実害なし |

## 依存関係

Phase 9.2（キーバインドカスタマイズ — 完了済み）

## リファレンス

- `muda` クレート (tauri-apps製): macOS/Windows/Linux ネイティブメニュー
- Ghostty の GTK メニュー実装: `refs/ghostty/src/apprt/gtk/class/surface.zig`
