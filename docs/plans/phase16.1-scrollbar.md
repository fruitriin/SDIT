# Phase 16.1: スクロールバー

**概要**: スクロールバック内の現在位置を視覚的に示す縦スクロールバーを実装する。

**状態**: **完了**

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| ScrollbarConfig 追加 | `enabled: bool`（デフォルト true）、`width: u8`（デフォルト 8px）、色設定 | sdit-core (`config/mod.rs`) | **完了** |
| スクロールバー位置計算 | サム位置 = display_offset / total_history、サムサイズ = viewport / total_lines | sdit (`render.rs`) | **完了** |
| wgpu 描画 | 右端にスクロールバートラック + サムを描画 | sdit (`render.rs`) | **完了** |
| マウスドラッグ操作 | スクロールバーのサムをドラッグでスクロール位置変更 | sdit (`scrollbar.rs`) | **完了** |
| スクロールバークリック | トラック部分クリックでページスクロール | sdit (`scrollbar.rs`) | **完了** |
| テスト | 位置計算のユニットテスト | sdit-core | **完了** |

## 設定例

```toml
[scrollbar]
enabled = true
width = 8
```

## 参照

- `refs/wezterm/wezterm-gui/src/scrollbar.rs` — スクロールバー実装
- `refs/ghostty/src/config/Config.zig` — scrollbar 設定
