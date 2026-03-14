# Phase 21.3: SGR 7 (INVERSE) の描画実装

## 問題

SDIT 上で ink (Claude Code TUI) の日本語レンダリングが欠ける。

## 原因

SGR 7 (INVERSE) のフラグ設定は `terminal/mod.rs` で実装済みだが、`pipeline.rs` の `update_from_grid` で `CellFlags::INVERSE` を参照して fg/bg を反転する処理が**未実装**。

ink は UI のハイライト・選択・プロンプト等に INVERSE を多用する。INVERSE が無視されると fg と bg が本来と逆のはずが反転されず、テキストが背景色と同色で描画されて「欠けて見える」。

## 修正方針

`pipeline.rs` の `update_from_grid` 内、セルの fg/bg を決定するロジックに INVERSE フラグのチェックを追加:

```rust
let (effective_fg, effective_bg) = if cell.flags.contains(CellFlags::INVERSE) {
    (cell.bg, effective_fg_color)  // fg と bg を入れ替え
} else {
    (effective_fg_color, cell.bg)
};
```

## 変更対象

- `crates/sdit-core/src/render/pipeline.rs` — `update_from_grid` の色決定ロジック

## テスト

- SGR 7 のエスケープシーケンスを出力して反転表示されることを確認
- 既存のカーソル反転・選択色との組み合わせが正しいことを確認

## 状態: 完了

## 実装内容

- `pipeline.rs` の `update_from_grid` で `CellFlags::INVERSE` 時に fg/bg を入れ替え
- カーソル・選択・URLホバー・通常描画すべてで `effective_bg` を使用するよう統一

## セキュリティレビュー結果

問題なし
