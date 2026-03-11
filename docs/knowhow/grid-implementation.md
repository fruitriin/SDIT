# Grid 実装知見

## 実装済みファイル構成

```
crates/sdit-core/src/
├── index.rs               # Point, Line, Column 座標型
└── grid/
    ├── mod.rs             # Grid<T> 本体、Scroll enum、Dimensions trait
    ├── cell.rs            # Cell, Color, NamedColor, CellFlags, GridCell trait
    ├── row.rs             # Row<T>（Vec<T> ラッパー + occ dirty tracking）
    └── storage.rs         # Storage<T> リングバッファ（O(1) スクロール）
```

## 設計判断

### `Line` は `i32`、`Column` は `usize`
- Line は負値（スクロールバック履歴）を表現するため `i32`
- Column は常に非負のため `usize`
- 演算は全て `saturating_*` を使用してオーバーフローを防止

### Storage のリングバッファ
- `zero` オフセットのみを更新することで O(1) スクロールを実現
- `compute_index(logical) = (zero + logical) % inner.len()`
- `rotate(count)` は `zero += count % len` のみ（物理メモリ移動なし）
- `rezero()` でコンパクト化（`rotate_left` 使用）

### `occ` フィールド（Row）
- 最後に書き込まれた列の上限インデックスを追跡
- `IndexMut` 実装内で自動更新
- `is_clear()` の早期脱出に使用

### `cast_possible_wrap` の扱い
- `usize → isize` キャストが必要な箇所（rotate の引数等）は
  `#[allow(clippy::cast_possible_wrap)]` でスコープを絞り、
  安全性の理由をコメントで文書化
- `usize → i32` は `i32::try_from(v).unwrap_or(i32::MAX)` で明示的変換

### scroll_up と履歴管理
- `region.start == 0` のとき、`rotate` でビューポート最上行を履歴に送る
- `raw.len` を手動インクリメントして履歴行をカウント
- `enforce_scroll_limit()` で `max_scroll_limit` 超過分を切り捨て

## テスト構成

各モジュールに `#[cfg(test)] mod tests` を配置。合計 45 テスト。

| モジュール | テスト数 | カバー範囲 |
|---|---|---|
| `index` | 5 | Line/Column 演算、saturating、Point 順序 |
| `cell` | 5 | is_empty, reset, bitflags, Color デフォルト |
| `row` | 8 | new, write/occ, is_clear, grow, shrink, reset |
| `storage` | 8 | len, rotate, swap, grow/shrink_visible, truncate |
| `grid` | 14 | 生成, cursor, scroll_up/down, display scroll, resize, clear |

## 注意点

- `Row::index_mut` は呼び出し側が境界チェックを行う前提（VTE 統合時に注意）
- `scroll_up` でサブリージョン（`region.start != 0`）の場合は履歴に積まない
- `clear_history` は `rotate` してから `len` を縮小するため `rezero` を内部で呼ぶ
- `truncate` は `rezero` 後にベクタを切り捨てる（物理的なメモリ解放）
