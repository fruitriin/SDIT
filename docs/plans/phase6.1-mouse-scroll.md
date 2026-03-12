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

## 実装結果（2026-03-12 完了）

**実装内容:**
- TermMode に5フラグ追加（MOUSE_REPORT_CLICK/DRAG/MOTION, SGR_MOUSE, UTF8_MOUSE）
- handler.rs: ?9/1000/1002/1003/1005/1006 のDECSET/DECRST対応
- input.rs: mouse_report_sgr(), mouse_report_x11(), pixel_to_grid() 追加
- event_loop.rs: マウスクリック/ドラッグ/ホイール → PTY報告（SGR/X11形式）
- ビューポートスクロール: ホイール + Shift+PageUp/Down
- PTY出力時のdisplay_offsetリセット（ライブビュー追従）
- テスト4件追加、115テスト全通過

**セキュリティレビュー結果:**
- M-1（修正済み）: スクロール行数の上限なしループ → `.clamp(1, 20)` で制限
- L-1: SGR button値の型安全性（u8で現状安全、将来拡張時に注意）
- L-2: X11座標クランプ（222超で切り詰め、仕様上の制約）
- L-3: UTF8_MOUSE フラグが定義のみで未実装（動作に影響なし）
- I-1: マウスモードON中のテキスト選択無効化（仕様通り、将来Shift+クリック検討）
- I-2: PTY出力時のスクロールバック強制リセット（一般的な設計）
