# vte 0.13 + Terminal 統合 ノウハウ

## vte::Params の扱い方

vte 0.13 の `Params` は「セミコロン区切りのトップレベルパラメータ」と「コロン区切りのサブパラメータ」の2階層を持つ。

```rust
// params.iter() → ParamsIter<'_>
// 各要素は &[u16]（サブパラメータ列）
for param in params {
    let top = param.first().copied().unwrap_or(0);  // トップレベル値
    let sub = param.get(1).copied().unwrap_or(0);   // サブパラメータ
}
```

SGR の 38/48（拡張色）はサブパラメータ方式 (`38:2:r:g:b`) と
トップレベル連続方式 (`38;2;r;g;b`) の両方に対応する必要がある。

## CSI パラメータのデフォルト値

多くの CSI シーケンスでは「パラメータ省略または 0 → デフォルト値」という規則がある。
`first_param(params, default)` / `nth_param(params, n, default)` ヘルパーで
「0 を渡した場合もデフォルトとして扱う」ことが正しい動作。

## 行数制限：テストコードも含むと500行を超える

`terminal/mod.rs` は Perform 実装本体 + ユニットテストを含むため 649 行になった。
ロジック本体（テスト除く）は約 440 行。CSI/ESC/SGR ハンドラを
`terminal/handler.rs` に分離することで両ファイルを管理可能な規模に保った。

## セキュリティ：erase_cells の start > end ガード

`erase_cells(start, end)` は start > end の場合に早期リターンが必要。
この防御を省くと、呼び出し側が逆順の範囲を渡した場合に
ループが画面全体を巡回し続ける（DoS 的な挙動）。

```rust
if start > end { return; }
```

## Alternate Screen Buffer の実装

`std::mem::swap` で `grid` と `inactive_grid` を入れ替えるだけで実装できる。
- 入る時: swap → clear_viewport + cursor リセット
- 出る時: swap だけでよい（primary の cursor は inactive_grid に保存されていた）

## Grid の cursor_cell() と IndexMut の使い分け

`cursor_cell()` は可変参照を返すが、同時に他のフィールドも参照しようとすると
借用チェッカーに引っかかる。そのため `print` 内では：

```rust
// NG: cursor_cell() のあとに template を読もうとするとコンパイルエラー
let cell = self.grid.cursor_cell();
cell.fg = self.grid.cursor.template.fg;  // ← self を二重借用

// OK: template をクローンしてから cursor_cell に書く
let tmpl = self.grid.cursor.template.clone();
let cell = self.grid.cursor_cell();
cell.fg = tmpl.fg;
```

## as i32 の cast_possible_wrap 警告

`usize as i32` は clippy pedantic で `cast_possible_wrap` 警告が出る。
`i32::try_from(x).unwrap_or(i32::MAX)` で回避する。
ターミナルの行/列は実用上 i32::MAX を超えないため `unwrap_or(i32::MAX)` で安全。
