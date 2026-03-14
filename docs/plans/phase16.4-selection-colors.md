# Phase 16.4: 選択色設定

**概要**: テキスト選択時のフォアグラウンド・バックグラウンド色をユーザーが設定できるようにする。

**状態**: 未着手

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| ColorConfig に selection_fg/bg 追加 | Optional、デフォルト None（反転色を使用） | sdit-core (`config/colors.rs`) | 未着手 |
| レンダリングで適用 | 選択セルの描画時に設定色を使用 | sdit (`render.rs`) | 未着手 |
| テスト | 設定デシリアライズ + デフォルト値 | sdit-core | 未着手 |

## 設定例

```toml
[colors]
selection_foreground = "#000000"
selection_background = "#FFFACD"
```

## 参照

- `refs/ghostty/src/config/Config.zig` — selection-foreground, selection-background
