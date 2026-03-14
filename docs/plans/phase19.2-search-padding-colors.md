# Phase 19.2: 検索ハイライト色 + パディング背景色

**概要**: スクロールバック検索結果のハイライト色と、ウィンドウパディング領域の背景色を設定可能にする。

**状態**: 完了

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| Config に search_foreground/background 追加 | Option<String> (hex color) | sdit-core (`config/color.rs`) | 完了 |
| Config に padding_color 追加 | "background" | sdit-core (`config/mod.rs`) | 完了 |
| 検索ハイライトの描画に色設定を反映 | render 時にカスタム色を使用 | sdit (`render.rs`) | 完了 |
| パディング領域の背景色描画 | PaddingColor::Background = 現在と同じ動作 | sdit (`render.rs`) | 完了 |
| テスト | 設定デシリアライズ | sdit-core | 完了 |

## 設定例

```toml
[colors]
search_foreground = "#ffffff"
search_background = "#ff8800"

[window]
padding_color = "background"  # "background" のみ対応（hex は将来拡張）
```

## 実装メモ

- `search_fg`/`search_bg` を `redraw_session` の前に設定からパースし、検索バー描画に渡す
- パディング色は `PaddingColor::Background` のみ（デフォルト動作と同じ）、将来 hex カラー拡張予定

## セキュリティ

- 色文字列は `parse_selection_color()` でパース（非 ASCII 拒否済み）
- パース失敗時は warn ログを出して None 扱い（デフォルト色を使用）

## 参照

- `refs/ghostty/src/config/Config.zig` — search-foreground, search-background, window-padding-color
