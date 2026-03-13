# Phase 13.3: 背景透過 + macOS blur

**概要**: ウィンドウ背景の不透明度設定。macOS ユーザーに人気の高い視覚カスタマイズ。

**状態**: 未着手

## タスク

| タスク | 詳細 | 変更先 |
|---|---|---|
| 背景アルファ設定 | `window.opacity: 0.0〜1.0` | sdit-core (`config/`) |
| wgpu クリアカラー | アルファチャンネルを反映 | sdit-core (`render/pipeline.rs`) |
| macOS blur | `NSVisualEffectView` 連携（winit raw handle 経由） | sdit (`render.rs`) |

## 参照

- `refs/alacritty/alacritty/src/config/window.rs`

## 依存関係

なし
