# Phase 15.3: フォント高度設定

**概要**: フォントフォールバックチェーン、フォントバリエーション軸、セル幅・高さ・ベースラインの微調整を実装する。

**状態**: **完了**

## 背景

- 日本語ユーザーや Nerd Fonts ユーザーにはフォントフォールバックが必須
- 可変フォント（Variable Font）のバリエーション軸設定は先進的ターミナルの標準機能になりつつある
- セル間隔の微調整はフォントによって必要になる

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| フォントフォールバック | font_family を Vec<String> に拡張、優先順位付き検索 | sdit-core (`config/mod.rs`, `font/`) | **完了** |
| font-codepoint-map | Unicode 範囲→フォント強制マッピング | sdit-core (`config/mod.rs`, `font/`) | **完了** |
| font-variation | 可変フォントの軸（wght, wdth 等）設定 | sdit-core (`font/`) | **完了** |
| font-feature | OpenType フィーチャー（calt, liga 等）の ON/OFF | sdit-core (`font/`) | **完了** |
| adjust-cell-width/height | セル幅・高さのピクセル/パーセンテージ調整 | sdit-core (`config/mod.rs`, `render/`) | **完了** |
| adjust-font-baseline | ベースライン位置の微調整 | sdit-core (`render/`) | **完了** |
| テスト | フォールバック解決 + 設定バリデーション | sdit-core | **完了** |

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

## 制限事項

- `variation` と `feature` は設定の読み込み・保存のみ実装。cosmic-text v0.12 には実行時適用の API がないため将来バージョン対応
- `fallback_families` は FontContext に保持するが、cosmic-text は自動でシステムフォールバックを処理するため明示的適用は不要

## セキュリティレビュー結果

| 重要度 | ID | 概要 | 対応 |
|---|---|---|---|
| Medium | M-1 | codepoint_map/variation/feature のエントリ数上限がデシリアライズ時に未適用 | **修正済み** — FontConfig::validate() 追加、Config::load() で呼び出し |
| Medium | M-2 | variation の f32 値に NaN/Inf チェックなし | **修正済み** — clamped_variation() で is_finite() フィルタ追加 |
| Low | L-1 | fallback_families のエントリ数・長さに上限なし | 実害は低い |
| Low | L-2 | ZWJ 等のゼロ幅文字で byte_to_col_map のカラムずれ | 描画の見た目のみ |
| Low | L-3 | byte_to_col.last().unwrap_or(0) のフォールバック理由がコメント不足 | 安全性に問題なし |
| Info | I-1 | family フィールドの文字列長上限がドキュメントに未記載 | ドキュメント改善 |
| Info | I-2 | parse_codepoint のプレフィックスなし 16 進数許容 | 仕様として許容 |

## 依存関係

なし（cosmic-text の既存機能を活用）
