# Phase 19.2: 検索ハイライト色 + パディング背景色

**概要**: スクロールバック検索結果のハイライト色と、ウィンドウパディング領域の背景色を設定可能にする。

**状態**: 未着手

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| Config に search_foreground/background 追加 | Option<String> (hex color) | sdit-core (`config/color.rs`) | 未着手 |
| Config に window_padding_color 追加 | "background" / hex color | sdit-core (`config/mod.rs`) | 未着手 |
| 検索ハイライトの描画に色設定を反映 | render 時にカスタム色を使用 | sdit (`render.rs`) | 未着手 |
| パディング領域の背景色描画 | clear_color の代わりにカスタム色を使用 | sdit (`render.rs`) | 未着手 |
| テスト | 設定デシリアライズ | sdit-core | 未着手 |

## 設定例

```toml
[colors]
search_foreground = "#ffffff"
search_background = "#ff8800"

[window]
padding_color = "background"  # "background" | "#rrggbb"
```

## 参照

- `refs/ghostty/src/config/Config.zig` — search-foreground, search-background, window-padding-color
