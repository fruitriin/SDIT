# 038: performable キーバインドの確認

## 概要

`performable = true` フラグ付きキーバインドが、アクションが実行可能なときのみキーを消費し、実行不可のときは PTY にキーを転送することを確認する。

## 前提条件

- SDIT がビルド・起動できる状態
- `~/.config/sdit/config.toml` に以下の設定を追加:

```toml
[[keybinds]]
key = "c"
mods = "super"
action = "Copy"
performable = true
```

## テスト手順

### ケース 1: performable = true でアクション実行可能（選択テキストあり）

1. SDIT を起動する
2. テキストを出力する（例: `echo hello`）
3. マウスドラッグで「hello」を選択する
4. `Cmd+C` を押す
5. **期待結果**:
   - 選択テキストがクリップボードにコピーされる（Copy アクションが実行される）
   - `c` の文字は PTY に送信されない（キーが消費される）

### ケース 2: performable = true でアクション実行不可（選択テキストなし）

1. SDIT を起動する（何も選択しない状態）
2. `Cmd+C` を押す
3. **期待結果**:
   - Copy アクションは実行されない（選択がないため）
   - キーが PTY に転送される（シェルに SIGINT 等が送られる）

### ケース 3: performable = false（デフォルト）で常にアクション実行

1. config.toml で `performable = true` を削除する（または `performable = false` に変更）
2. SDIT を再起動（または Hot Reload）
3. 何も選択しない状態で `Cmd+C` を押す
4. **期待結果**:
   - Copy アクションが実行される（何もコピーされないが、キーは消費される）
   - PTY にはキーが転送されない

### ケース 4: TOML デシリアライズ確認

1. config.toml に `performable = true` を含むバインディングを追加
2. SDIT を起動する
3. **期待結果**:
   - エラーなく起動する
   - `performable` フィールドが正しくパースされる

## ユニットテスト対応

- `crates/sdit/src/input.rs` の `resolve_action_performable_true` / `resolve_action_performable_false_default` / `resolve_action_performable_mixed` テストでロジックを検証
