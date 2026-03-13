# Phase 13.2: ビジュアルベル + システム通知

**概要**: BEL (0x07) 受信時の視覚フィードバック。現在はログ出力のみ。

**状態**: 未着手

## タスク

| タスク | 詳細 | 変更先 |
|---|---|---|
| ビジュアルベル | 画面を一瞬フラッシュ（背景色反転→フェード） | sdit (`render.rs`) |
| macOS Dock バウンス | `NSApp::requestUserAttention()` で通知 | sdit (`event_loop.rs`) |
| 設定項目 | `bell.visual = true/false`, `bell.dock_bounce = true/false` | sdit-core (`config/`) |

## 参照

- `refs/alacritty/alacritty/src/display/bell.rs`

## 依存関係

なし
