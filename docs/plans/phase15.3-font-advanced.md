# Phase 15.3: フォント高度設定

**概要**: フォントフォールバックチェーン、フォントバリエーション軸、セル幅・高さ・ベースラインの微調整を実装する。

**状態**: 未着手

## 背景

- 日本語ユーザーや Nerd Fonts ユーザーにはフォントフォールバックが必須
- 可変フォント（Variable Font）のバリエーション軸設定は先進的ターミナルの標準機能になりつつある
- セル間隔の微調整はフォントによって必要になる

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| フォントフォールバック | font_family を Vec<String> に拡張、優先順位付き検索 | sdit-core (`config/mod.rs`, `font/`) | 未着手 |
| font-codepoint-map | Unicode 範囲→フォント強制マッピング | sdit-core (`config/mod.rs`, `font/`) | 未着手 |
| font-variation | 可変フォントの軸（wght, wdth 等）設定 | sdit-core (`font/`) | 未着手 |
| font-feature | OpenType フィーチャー（calt, liga 等）の ON/OFF | sdit-core (`font/`) | 未着手 |
| adjust-cell-width/height | セル幅・高さのピクセル/パーセンテージ調整 | sdit-core (`config/mod.rs`, `render/`) | 未着手 |
| adjust-font-baseline | ベースライン位置の微調整 | sdit-core (`render/`) | 未着手 |
| テスト | フォールバック解決 + 設定バリデーション | sdit-core | 未着手 |

## 設定例

```toml
[font]
family = ["JetBrains Mono", "Noto Sans CJK JP", "Symbols Nerd Font"]
# codepoint_map = { "U+3000-U+9FFF" = "Noto Sans CJK JP" }
variation = { wght = 400 }
feature = { calt = true, liga = true }

[font.adjust]
cell_width = 0     # ピクセル加算
cell_height = 0
baseline = 0
```

## 参照

- `refs/ghostty/src/font/Collection.zig` — フォールバックチェーン
- `refs/ghostty/src/font/CodepointMap.zig` — コードポイントマッピング
- `refs/ghostty/src/font/Metrics.zig` — MetricModifier
- `refs/ghostty/src/config/Config.zig` — font-variation, font-feature, adjust-*

## 依存関係

なし（cosmic-text の既存機能を活用）
