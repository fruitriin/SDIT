# Phase 6.1: マウスイベント報告 + スクロールUI

**概要**: TUIアプリ(vim, htop, lazygit等)の日常使いに必須のマウスイベント報告とスクロールUIを実装する。

## タスク

| タスク | 詳細 | 変更先クレート |
|---|---|---|
| TermMode にマウスモード追加 | `MOUSE_REPORT_CLICK`, `MOUSE_REPORT_DRAG`, `MOUSE_REPORT_MOTION`, `SGR_MOUSE`, `UTF8_MOUSE` を `TermMode` bitflags に追加 | sdit-core |
| CSI DECSET/DECRST にマウスモード処理追加 | `?9`, `?1000`, `?1002`, `?1003`, `?1006` の set/reset を `set_private_mode` に追加 | sdit-core (`handler.rs`) |
| main.rs にマウスイベントディスパッチ追加 | `WindowEvent::MouseInput`, `CursorMoved`, `MouseWheel` をマウスモードに応じてPTYにSGR/X11形式で報告 | sdit (`main.rs`) |
| ビューポートスクロール | マウスホイール(マウスモードOFF時)・Shift+PageUp/Down でスクロールバック閲覧 | sdit-core (`grid/mod.rs`), sdit (`main.rs`) |

## 依存関係

なし

## リファレンス

- `refs/alacritty/alacritty_terminal/src/term/mod.rs` — TermMode のマウスフラグ定義
- `refs/alacritty/alacritty/src/input/mod.rs` — マウスイベントからPTYバイト列への変換

## セキュリティ考慮事項

- マウス座標の境界チェック
- 悪意あるアプリがマウスモードを意図的にONにしてユーザー操作を妨害するリスク
