# GUI テストシナリオ

## 概要

GUI テストのシナリオを markdown で管理する。
サブエージェントがこのディレクトリのシナリオを読み、`tools/test-utils/` を使ってテストを実行する。

## シナリオファイルの形式

```markdown
# シナリオ名

## 前提条件
- ビルド済み: `cargo build --package sdit`
- test-utils ビルド済み: `tools/test-utils/build.sh`
- 権限付与済み（必要な場合）

## 手順
1. SDIT をバックグラウンドで起動する
2. window-info でウィンドウの存在を確認する
3. send-keys で "echo hello" を入力する
4. send-keys で Enter キーを送信する
5. 1秒待機する
6. capture-window でスクリーンショットを撮る

## 期待結果
- ウィンドウが表示されている
- スクリーンショットに "hello" が描画されている

## クリーンアップ
- SDIT プロセスを終了する
- スクリーンショットファイルを削除する
```

## 命名規約

`NNN-シナリオ名.md`（例: `001-basic-echo.md`, `002-key-input.md`）

## テスト記述上の注意

### send-keys.sh でのコマンド入力

コマンドと引数のスペースは **1つの文字列に含める**こと:

```bash
# ✅ 正しい
./tools/test-utils/send-keys.sh sdit "echo hello world"

# ❌ 誤り（"echohello world" と結合される）
./tools/test-utils/send-keys.sh sdit "echo"
./tools/test-utils/send-keys.sh sdit "hello world"
```

CJK 文字など IME が干渉する文字列は `printf "...\r" > /dev/ttysNNN`（PTY 直接書き込み）が安定。
詳細は `docs/knowhow/integration-testing-patterns.md` の「シェルコマンド入力のスペース問題」を参照。
