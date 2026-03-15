# 037: パレット自動生成（palette_generate / palette_harmonious）

## 概要

`palette_generate = true` で bg/fg から ANSI 16色パレットを HSL 補間で自動生成し、
`palette_harmonious = true` で明暗テーマの自動適応（normal/bright の lightness 入れ替え）を確認する。

## 前提条件

- SDIT がビルド・起動できる状態
- カスタムテーマを設定可能な `config.toml`

## テスト手順

### ケース 1: palette_generate = true でカスタムパレット生成

1. `~/.config/sdit/config.toml` に以下を設定:

```toml
[colors]
theme = "custom"
palette_generate = true

[colors.custom]
background = "#1a1b26"
foreground = "#a9b1d6"
```

2. SDIT を起動する
3. ANSI カラーを出力するコマンドを実行（例: カラーテスト用のシェルスクリプト）:

```bash
for i in $(seq 0 15); do printf "\033[48;5;${i}m  \033[0m"; done; echo
```

4. **期待結果**:
   - 16色が背景色 `#1a1b26` と前景色 `#a9b1d6` の色相・明度に基づいて自動生成された色で表示される
   - テーマのデフォルトパレット（Catppuccin 等）とは異なる色になる

### ケース 2: palette_generate = false（デフォルト）ではテーマ固有パレット使用

1. `palette_generate = true` の行を削除（または `false` に変更）
2. SDIT を再起動（または Hot Reload）
3. 同じカラーテストコマンドを実行

4. **期待結果**:
   - テーマ固有のパレット色（各テーマに定義済みの正確な ANSI 16色）が使用される

### ケース 3: palette_harmonious = true で明暗適応

1. 以下の設定にする:

```toml
[colors]
theme = "custom"
palette_generate = true
palette_harmonious = true

[colors.custom]
background = "#fafafa"
foreground = "#383a42"
```

2. SDIT を起動しカラーテストを実行

3. **期待結果**:
   - ライトテーマ（明るい背景）に適した色調になる
   - normal 色と bright 色の明度が入れ替えられている（暗いテーマ用の生成結果とは明暗が逆転）

### ケース 4: Hot Reload でパレット変更が反映される

1. `palette_generate = false` で SDIT を起動
2. カラーテストコマンドを実行し、テーマ固有パレットを確認
3. 設定ファイルを編集して `palette_generate = true` に変更
4. Hot Reload が発火するのを待つ

5. **期待結果**:
   - パレットが自動生成されたものに即座に変わる
   - 既に表示されているテキストの色も更新される

### ケース 5: palette_harmonious は palette_generate なしでは無効

1. 以下の設定にする:

```toml
[colors]
palette_generate = false
palette_harmonious = true
```

2. SDIT を起動しカラーテストを実行

3. **期待結果**:
   - テーマ固有パレットが使用される（harmonious は generate が true でないと効果がない）

## ユニットテスト対応

- `crates/sdit-core/src/config/color.rs` のテスト:
  - `generate_ansi_palette` が 16色を生成すること
  - `apply_harmonious` が normal/bright の lightness を入れ替えること
  - `from_color_config` が `palette_generate` フラグに応じてパレットを切り替えること
  - 各テーマの `ansi_palette` が正しく定義されていること
