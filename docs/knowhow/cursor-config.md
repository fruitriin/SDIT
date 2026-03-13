# カーソル設定実装ノウハウ

## 実装概要 (Phase 14.1)

`[cursor]` TOML セクションでカーソルスタイル・点滅・色を設定可能にした。

## 設計ポイント

### CursorStyleConfig — serde 用と内部型の分離

`terminal::CursorStyle` は serde に依存しないコア型。設定ファイル用に `config::CursorStyleConfig` を別定義し、`From<CursorStyleConfig> for CursorStyle` で変換する。

```rust
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CursorStyleConfig {
    #[default] Block, Underline, Bar,
}
impl From<CursorStyleConfig> for CursorStyle { ... }
```

### Terminal のデフォルトカーソル管理

`cursor_style` / `cursor_blinking` はアプリ（neovim 等）が DECSCUSR で変更する値。
`default_cursor_style` / `default_cursor_blinking` は設定ファイル由来の基準値で、DECSCUSR 0 で使用される。

- `Terminal::new_with_cursor(lines, cols, scrollback, style, blinking)` で初期値ごとセットする
- `Terminal::set_default_cursor(style, blinking)` は hot reload 時にデフォルトのみ更新する（現在のアプリスタイルは変えない）

### DECSCUSR 0 の意味

DECSCUSR 0 は「ターミナルデフォルトに戻す」シーケンス。
以前は `Block + blinking = true` にハードコードされていたが、`default_cursor_style` / `default_cursor_blinking` を使うよう修正。

### DECSCUSR シーケンスの形式

正規形は `CSI params SP q` = `\x1b[Nq`（N はスタイル番号、SP は中間バイト 0x20）。
テストでは `\x1b[1 q` (params=1, intermediate=' ') を使う（`\x1b[ 1q` ではなく）。

| N | スタイル | 点滅 |
|---|---|---|
| 0 | デフォルト（設定値） | デフォルト |
| 1 | Block | あり |
| 2 | Block | なし |
| 3 | Underline | あり |
| 4 | Underline | なし |
| 5 | Bar | あり |
| 6 | Bar | なし |

### カーソル色描画

`CursorConfig.color` が `Some(hex)` の場合、`pipeline.rs` の `update_from_grid` に `cursor_color: Option<[f32; 4]>` として渡す。
カーソルセルでは反転（fg↔bg）ではなく、その色を背景色として使用する。

`parse_hex_color(hex: &str) -> Option<[f32; 4]>` は `render.rs` で定義:
- `#rrggbb` 形式のみサポート
- パース失敗時は `None`（呼び出し側でログ警告して None 扱い）

### SessionManager::all() の追加

hot reload で全セッションを走査するため `SessionManager::all()` を追加:
```rust
pub fn all(&self) -> impl Iterator<Item = &Session> { self.sessions.values() }
```

### terminal/mod.rs の行数管理

1500 行上限のため、テストモジュールを `terminal/tests.rs` に分離した:
- `mod.rs` に `#[cfg(test)] mod tests;` を記述
- `tests.rs` に全テストを移動
- `tests.rs` は `use super::*` で `pub(super)` フィールドにアクセス可能

新規テスト（DECSCUSR / カーソル設定）は `handler.rs` の `mod tests` に配置。
