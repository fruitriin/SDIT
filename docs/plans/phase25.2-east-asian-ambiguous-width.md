# Phase 25.2: 東アジア曖昧幅文字の広幅扱い設定

## 要望

Unicode の「East Asian Ambiguous Width」文字（○、□、★、→、℃ 等）を
2セル幅として描画するかどうかを設定できるようにする。

日本語・中国語・韓国語環境ではこれらを「全角」として扱う慣習があり、
多くの CJK ターミナルエミュレータはデフォルトで広幅扱いにしている。

WezTerm: `treat_east_asian_ambiguous_width_as_wide = true`
Ghostty: `grapheme-width-method = unicode` (Phase 19.4 で対応済み) + 曖昧幅制御

## 現状

Phase 19.4 で `grapheme-width-method` を実装済みだが、これは結合文字・絵文字の幅計算方法であり、
East Asian Ambiguous Width の個別制御ではない。

CJK ユーザーが `unicode-width` の Ambiguous 文字を 1 セルで表示するか 2 セルで表示するかは
環境依存で、現在 SDIT では固定されている。

## 実装方針

1. `[terminal] east_asian_ambiguous_width = "narrow" | "wide"` 設定を追加
2. `narrow`（デフォルト）: 現行動作（unicode-width が 1 を返す文字は 1 セル）
3. `wide`: 東アジア曖昧幅文字を強制 2 セル扱い
4. `sdit-core/src/grid/` または VTE パーサー内でセル幅計算時に設定を参照

## 変更対象

- `crates/sdit-core/src/config/mod.rs` — `[terminal] east_asian_ambiguous_width` 追加
- `crates/sdit-core/src/terminal/` — セル幅計算時に設定値を参照

## 実装結果（2026-03-15 完了）

- `EastAsianAmbiguousWidth` enum（Narrow/Wide）を config に追加
- `Terminal::print()` で Wide 時に `width_cjk()` を使用（Ambiguous 文字を 2 セル扱い）
- `.min(2)` の防御的ガードを追加（M-1 セキュリティ修正）
- ホットリロード対応済み

テスト: 444 件 PASS（ユニットテスト 3 件追加）
セキュリティ: M-1 修正済み

## セキュリティ影響

なし（セキュリティレビュー済み・M-1 修正完了）

## 参照

- WezTerm: `refs/wezterm/config/src/config.rs` `treat_east_asian_ambiguous_width_as_wide`
- Unicode: https://www.unicode.org/reports/tr11/ East Asian Width
