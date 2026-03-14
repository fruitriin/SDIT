# 032: 東アジア曖昧幅文字のセル幅設定確認

## 目的
`[terminal] east_asian_ambiguous_width` 設定により、曖昧幅文字（○□★→℃ 等）のセル幅が
1 セル（Narrow）と 2 セル（Wide）で切り替わることを確認する。

## 前提条件
- `cargo build --package sdit`
- `cargo test -p sdit-core` が PASS

## ユニットテスト

### テストケース一覧

| テスト名 | 内容 |
|---|---|
| `east_asian_ambiguous_narrow_circle` | Narrow モードで ○（U+25CB）が幅1 |
| `east_asian_ambiguous_wide_circle` | Wide モードで ○（U+25CB）が幅2 |
| `east_asian_ambiguous_wide_cjk_unchanged` | Wide モードで あ（U+3042）が幅2のまま |

### 実行コマンド
```bash
cargo test -p sdit-core east_asian_ambiguous
```

### 検証ポイント

#### Narrow モード（デフォルト）
- ○（U+25CB）のセルに `WIDE_CHAR` フラグがない
- カーソルが列1に進む（幅1）

#### Wide モード
- ○（U+25CB）のセルに `WIDE_CHAR` フラグがある
- 隣接セル（列1）に `WIDE_CHAR_SPACER` フラグがある
- カーソルが列2に進む（幅2）
- 通常の CJK 文字（あ U+3042）は影響なし（幅2のまま）

## GUI テスト（任意）

### 手順
1. `~/.config/sdit/config.toml` に以下を設定:
   ```toml
   [terminal]
   east_asian_ambiguous_width = "wide"
   ```
2. SDIT を起動
3. `echo "○□★→℃"` を実行
4. capture-window でスクリーンショットを撮る
5. 各文字が 2 セル幅で表示されていることを目視確認
6. 設定を `"narrow"` に変更して SDIT を再起動
7. 同じ echo を実行し、1 セル幅で表示されていることを確認

### 期待結果
- Wide モード: 曖昧幅文字が全角幅（CJK 文字と同じ幅）で表示される
- Narrow モード: 曖昧幅文字が半角幅で表示される

## 設定デシリアライズ確認

```toml
# デフォルト（省略時）→ Narrow
[terminal]

# 明示的に Narrow
[terminal]
east_asian_ambiguous_width = "narrow"

# Wide
[terminal]
east_asian_ambiguous_width = "wide"
```

## 関連
- Phase 25.2: East Asian Ambiguous Width 設定
- `crates/sdit-core/src/terminal/mod.rs` — `Terminal::print()` の幅判定
- `crates/sdit-core/src/config/mod.rs` — `EastAsianAmbiguousWidth` enum
- 009-cjk-display — CJK 全角文字の基本描画シナリオ
