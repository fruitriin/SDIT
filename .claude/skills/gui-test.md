---
name: gui-test
description: GUI テストシナリオを実行する。docs/test-scenarios/ のシナリオファイルを読み、tools/test-utils/ を使ってテストを実施する。
user_invocable: true
---

# GUI テスト実行

## 引数
- `$ARGUMENTS`: シナリオ番号（例: "001"）またはシナリオファイル名。省略時は全シナリオを一覧表示。

## 手順

### 引数なしの場合
1. `docs/test-scenarios/` 内の全 `.md` ファイル（README.md 除く）を一覧表示する
2. 各ファイルの `# ` 見出しからシナリオ名を抽出して表示する

### シナリオ指定の場合
1. `docs/test-scenarios/` から該当するシナリオファイルを読む
2. `tools/test-utils/CLAUDE.md` を読み、ツールの使い方を確認する
3. シナリオの「前提条件」を確認する:
   - test-utils がビルド済みか確認（`tools/test-utils/window-info` の存在チェック）
   - 未ビルドなら `tools/test-utils/build.sh` を実行する
4. シナリオの「手順」に従ってテストを実行する:
   - SDIT の起動: `cargo run --package sdit &`
   - 各ツールの呼び出し: `tools/test-utils/` のツールを使用
   - 一時ファイルは `tmp/` に書き出す（`/tmp/` は使用禁止）
5. 「期待結果」と実際の結果を比較する
6. 「クリーンアップ」を実行する
7. 結果を報告する（成功/失敗 + 詳細）

## 注意事項
- GUI テストはディスプレイ環境が必要
- Screen Recording / Accessibility 権限が必要な場合がある
- 失敗した場合はスクリーンショットを `tmp/` に保存して報告する
- SDIT プロセスは必ずクリーンアップで終了させる
