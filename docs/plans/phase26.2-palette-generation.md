# Phase 26.2: パレット自動生成（Palette Generate / Harmonious）

## 要望

現在のテーマカラー（background / foreground）から ANSI 16 色パレットを自動生成する機能を追加する。
カスタムテーマを 2 色だけ指定すれば、残りの色が自動的に補間・調整される。

Ghostty:
- `palette-generate = true`: background から 6×6×6 色立方体 + グレースケールを補間
- `palette-harmonious = true`: 生成パレットの明暗を逆順にして暗・明テーマの自動適応

## 動作イメージ

```toml
[colors]
theme = "custom"
[colors.custom]
background = "#1a1b26"
foreground = "#a9b1d6"
palette_generate = true   # ← 追加
```

background / foreground から 256 色パレットの補間値を自動計算する。
`palette_harmonious = true` でさらに明暗を調整する。

## 実装方針

1. `ThemeConfig` または `ColorsConfig` に `palette_generate: bool`、`palette_harmonious: bool` を追加
2. `ResolvedColors::from_theme()` でカラー解決時に、`palette_generate = true` ならば
   background / foreground を基にパレット色を補間生成する
3. 補間アルゴリズム: HSL 色空間で linearly interpolated 256 色

## 変更対象

- `crates/sdit-core/src/config/color.rs` — `ResolvedColors` 生成時のパレット補間ロジック
- `crates/sdit-core/src/config/mod.rs` — `palette_generate / palette_harmonious` 設定追加

## セキュリティ影響

なし

## 参照

- Ghostty: `refs/ghostty/src/config/Config.zig` L815 `palette-generate`, L833 `palette-harmonious`
