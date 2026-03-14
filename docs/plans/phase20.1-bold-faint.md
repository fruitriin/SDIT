# Phase 20.1: Bold as Bright + Faint Opacity

**概要**: 太字テキストを明色に変換する機能と、暗字（SGR 2）の透明度を調整する機能を追加する。

**状態**: 完了

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| Config に bold_is_bright 追加 | bool (デフォルト false) | sdit-core (`config/color.rs`) | 完了 |
| Config に faint_opacity 追加 | f32 (デフォルト 0.5, 0.0〜1.0) | sdit-core (`config/color.rs`) | 完了 |
| レンダリングで bold_is_bright を反映 | BOLD フラグ + Named color → Bright variant | sdit-core (`render/pipeline.rs`) | 完了 |
| レンダリングで faint_opacity を反映 | DIM フラグ時のアルファ値調整 | sdit-core (`render/pipeline.rs`) | 完了 |
| テスト | 設定デシリアライズ + クランプ | sdit-core | 完了 |

## 設定例

```toml
[colors]
bold_is_bright = false  # true にすると SGR 1 + Named color → Bright variant
faint_opacity = 0.5     # SGR 2 (DIM) のアルファ値 (0.0〜1.0)
```

## セキュリティレビュー結果

| 重要度 | ID | 内容 | 対応 |
|---|---|---|---|
| Low | L-1 | faint_opacity のデシリアライズ時バリデーション | 記録のみ: clamped_faint_opacity() で安全 |
| Info | I-1 | bold_is_bright は Named 色のみ変換対象 | 記録のみ: 仕様通り |

## 参照

- `refs/ghostty/src/config/Config.zig` — bold-is-bright, faint-opacity
- `refs/alacritty/` — draw_bold_text_with_bright_colors
