# Phase 20.1: Bold as Bright + Faint Opacity

**概要**: 太字テキストを明色に変換する機能と、暗字（SGR 2）の透明度を調整する機能を追加する。

**状態**: 未着手

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| Config に bold_is_bright 追加 | bool (デフォルト false) | sdit-core (`config/color.rs`) | 未着手 |
| Config に faint_opacity 追加 | f32 (デフォルト 0.5, 0.0〜1.0) | sdit-core (`config/color.rs`) | 未着手 |
| レンダリングで bold_is_bright を反映 | BOLD フラグ + Named color → Bright variant | sdit (`render.rs`) | 未着手 |
| レンダリングで faint_opacity を反映 | DIM フラグ時のアルファ値調整 | sdit (`render.rs`) | 未着手 |
| テスト | 設定デシリアライズ + クランプ | sdit-core | 未着手 |

## 設定例

```toml
[colors]
bold_is_bright = false  # true にすると SGR 1 + Named color → Bright variant

[colors]
faint_opacity = 0.5  # SGR 2 (DIM) のアルファ値 (0.0〜1.0)
```

## 参照

- `refs/ghostty/src/config/Config.zig` — bold-is-bright, faint-opacity
- `refs/alacritty/` — draw_bold_text_with_bright_colors
