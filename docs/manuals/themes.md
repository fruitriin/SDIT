# テーマ

## 組み込みテーマ

| テーマ名 | 系統 | 背景色 | 前景色 |
|---|---|---|---|
| `catppuccin-mocha` | ダーク | #1e1e2e | #cdd6f4 |
| `catppuccin-latte` | ライト | #eff1f5 | #4c4f69 |
| `gruvbox-dark` | ダーク | #282828 | #ebdbb2 |
| `solarized-dark` | ダーク | #002b36 | #839496 |
| `solarized-light` | ライト | #fdf6e3 | #586e75 |
| `dracula` | ダーク | #282a36 | #f8f8f2 |
| `nord` | ダーク | #2e3440 | #d8dee9 |
| `one-dark` | ダーク | #282c34 | #abb2bf |
| `tokyo-night` | ダーク | #1a1b26 | #c0caf5 |

すべてのテーマは WCAG AA 基準（コントラスト比 4.5:1 以上）を満たしています。

## テーマの設定

```toml
[colors]
theme = "dracula"
```

## テーマの切り替え

キーバインドでテーマをサイクル切り替えできます。

```toml
[[keybinds]]
key = "t"
mods = "ctrl|shift"
action = "NextTheme"
```

`NextTheme` で次のテーマ、`PreviousTheme` で前のテーマに切り替わります。

## 選択色・検索色のカスタマイズ

テーマの選択色や検索ハイライト色を個別に上書きできます。

```toml
[colors]
theme = "catppuccin-mocha"
selection_foreground = "#1e1e2e"
selection_background = "#f5e0dc"
search_foreground = "#1e1e2e"
search_background = "#f9e2af"
```

## コントラスト調整

WCAG 基準に基づいた最小コントラスト比を強制できます。

```toml
[colors]
minimum_contrast = 4.5   # WCAG AA 基準
```

`1.0` に設定すると無効になります（デフォルト）。
