# 036: unconsumed キーバインドの確認

## 概要

`unconsumed = true` フラグ付きキーバインドが、アクションを実行しつつ元のキーを PTY にも転送することを確認する。

## 前提条件

- SDIT がビルド・起動できる状態
- `~/.config/sdit/config.toml` に以下の設定を追加:

```toml
[[keybinds]]
key = "k"
mods = "super"
action = "ScrollToBottom"
unconsumed = true
```

## テスト手順

### ケース 1: unconsumed = true でアクション実行 + PTY 転送

1. SDIT を起動する
2. 数行分のテキストを出力してスクロール可能な状態にする（例: `seq 200`）
3. 上方向にスクロールする（マウスホイール or PageUp）
4. `Cmd+K` を押す
5. **期待結果**:
   - スクロールが最下部に戻る（ScrollToBottom アクションが実行される）
   - かつ、`k` の文字が PTY に送信される（シェルプロンプトに `k` が入力される）

### ケース 2: unconsumed = false（デフォルト）でアクション実行のみ

1. config.toml で `unconsumed = true` を `unconsumed = false` に変更（または行を削除）
2. SDIT を再起動（または Hot Reload）
3. 同様に上方向にスクロールした後、`Cmd+K` を押す
4. **期待結果**:
   - スクロールが最下部に戻る（ScrollToBottom アクションが実行される）
   - `k` の文字は PTY に送信されない（シェルプロンプトに `k` は入力されない）

### ケース 3: unconsumed 設定なし（デフォルト値 = false）

1. config.toml から `unconsumed` 行を完全に削除する
2. SDIT を再起動
3. 同様にスクロール後 `Cmd+K` を押す
4. **期待結果**:
   - ケース 2 と同じ（デフォルトは consumed = アクションのみ）

## ユニットテスト対応

- `crates/sdit/src/input.rs` の `resolve_action_unconsumed_true` / `resolve_action_unconsumed_false_default` / `resolve_action_unconsumed_mixed` テストでロジックを検証
