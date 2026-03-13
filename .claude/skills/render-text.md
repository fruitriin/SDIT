---
name: render-text
description: CoreText で対照群テキスト画像を生成する。CJK/絵文字/リガチャの表示品質テストの「正解画像」を作る。
user_invocable: true
---

# 対照群テキスト画像の生成

## 引数
- `$ARGUMENTS`: レンダリングするテキスト。オプションも指定可能。
  - 例: `"こんにちは世界"`
  - 例: `--font Monaco --size 18 "テスト文字列"`
  - 省略時: 使い方を表示する

## 手順

### 引数なしの場合
以下の使い方を表示する:

```
render-text [options] <text>

Options:
  --font <name>    フォント名 (default: Menlo)
  --size <pt>      フォントサイズ (default: 14)
  --bg <hex>       背景色 (default: 282828)
  --fg <hex>       テキスト色 (default: EBDBB2)
  --mono           等幅グリッド描画（ターミナル風、推奨）
  --cell-info      セル境界座標を JSON で出力
  --scale <n>      Retina スケール (default: 2)

出力先: tmp/render-text-output.png
```

### テキスト指定の場合

1. `tools/test-utils/render-text` がビルド済みか確認する（バイナリの存在チェック）
   - 未ビルドなら `tools/test-utils/build.sh` を実行する

2. 出力ファイル名を決定する:
   - `tmp/render-text-output.png`（デフォルト）
   - `--cell-info` 指定時は JSON も `tmp/render-text-cells.json` に保存する

3. コマンドを組み立てて実行する:
   ```bash
   ./tools/test-utils/render-text --mono --cell-info $ARGUMENTS tmp/render-text-output.png
   ```
   - `--mono` はデフォルトで付与する（ターミナル比較用途がほとんどのため）
   - `--cell-info` もデフォルトで付与する（verify-text との連携のため）
   - ユーザーが明示的に `--mono` なしを指定した場合はそれに従う

4. `--cell-info` の JSON 部分を分離して保存する:
   ```bash
   # 1行目は "Rendered: ..." なので2行目以降が JSON
   tail -n +2 < 出力 > tmp/render-text-cells.json
   ```

5. 生成した画像を Read ツールで表示してユーザーに見せる

6. 結果を報告する:
   - 画像パス: `tmp/render-text-output.png`
   - セル境界 JSON パス: `tmp/render-text-cells.json`（cell-info 使用時）
   - 画像サイズ（px）

## 注意事項
- 出力先は `tmp/` を使う（`/tmp/` は使用禁止）
- SDIT の設定（フォント・サイズ・色）と揃えたい場合は `crates/sdit-core/src/config/` のデフォルト値を参照する
