# Phase 16.8: 最小コントラスト比

**概要**: 前景色と背景色のコントラスト比が指定値を下回る場合にテキスト色を自動調整してアクセシビリティを確保する。

**状態**: 未着手

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| ColorConfig に minimum_contrast 追加 | f32（1.0-21.0、デフォルト 1.0 = 無効） | sdit-core (`config/colors.rs`) | 未着手 |
| 相対輝度計算 + コントラスト調整 | WCAG 2.0 準拠の相対輝度計算、コントラスト不足時に fg 調整 | sdit-core (`render/`) or sdit (`render.rs`) | 未着手 |
| テスト | 相対輝度計算 + コントラスト調整のユニットテスト | sdit-core | 未着手 |

## 設定例

```toml
[colors]
minimum_contrast = 4.5  # WCAG AA 基準
```

## 参照

- `refs/ghostty/src/config/Config.zig` — minimum-contrast
