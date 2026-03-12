# Phase 11.1: macOS メニューバー

**概要**: macOS ネイティブメニューバーを実装する。winit 0.30 にはメニューAPIがないため、`muda` クレートまたは `cocoa` 直接呼び出しで実装する。

## タスク

| タスク | 詳細 | 変更先クレート |
|---|---|---|
| muda クレート統合 | macOS ネイティブメニューバーの基盤を導入 | sdit (`main.rs`), `Cargo.toml` |
| アプリケーションメニュー | SDIT > About, Preferences, Quit | sdit |
| ファイルメニュー | New Window (Cmd+N), New Tab (Cmd+T), Close (Cmd+W) | sdit |
| 編集メニュー | Copy (Cmd+C), Paste (Cmd+V), Select All (Cmd+A) | sdit |
| 表示メニュー | Toggle Sidebar (Cmd+\\), Font Size +/- (Cmd+=/-)  | sdit |
| メニューアクションとキーバインドの統合 | `is_*_shortcut()` 関数群と Menu Action を統一的に管理 | sdit |

## 依存関係

Phase 9.2（キーバインドカスタマイズ。メニューとショートカットの一元管理が前提）

## リファレンス

- `muda` クレート (tauri-apps製): macOS/Windows/Linux ネイティブメニュー
- Ghostty の GTK メニュー実装: `refs/ghostty/src/apprt/gtk/class/surface.zig`

## 注意

Alacritty, WezTerm ともにメニューバーは実装していない（ショートカット駆動の設計哲学）。SDITはSDIファーストでウィンドウ単位の操作が多いため、メニューバーは操作の発見性（discoverability）向上に有効。

## 新規依存クレート

`muda`
