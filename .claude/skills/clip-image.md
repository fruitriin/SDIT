---
name: clip-image
description: PNG 画像の指定領域を切り出す。annotate-grid で座標確認後、注目領域だけを LLM に渡す際に使う。
user_invocable: true
---

# 画像クリップ

## 引数
- `$ARGUMENTS`: `<画像パス> [出力パス] [options]`
  - 例: `tmp/capture.png --grid-cell 3 2 8`
  - 例: `tmp/capture.png tmp/clip.png --rect 100 200 400 300`
  - 省略時: 使い方を表示する

## 手順

### 引数なしの場合
以下の使い方を表示する:

```
clip-image <input-png> <output-png> --rect x y width height
clip-image <input-png> <output-png> --grid-cell col row N

クリップ領域（いずれか1つ必須）:
  --rect x y width height    ピクセル座標で矩形指定（左上原点）
  --grid-cell col row N      annotate-grid --divide N と同じ分割で col,row を切り出す
                              col, row は 0-origin（左上が 0,0）

Exit: 0=成功, 1=エラー
```

### 引数ありの場合

1. `tools/test-utils/clip-image` がビルド済みか確認する
   - 未ビルドなら `cd tools/test-utils && ./build.sh` を実行する

2. 入力ファイルが存在するか確認する
   - 存在しない場合、`tmp/` 内の最新の PNG を候補として提示する

3. 出力パスを決定する:
   - 引数に出力パスが含まれていれば使用する
   - 含まれていない場合は `tmp/clip-<入力ファイル名>` を自動設定する
     - 例: 入力が `tmp/capture.png` → 出力 `tmp/clip-capture.png`

4. コマンドを組み立てて実行する:
   ```bash
   ./tools/test-utils/clip-image <input> <output> <options>
   ```

5. 結果を報告する:
   - 出力ファイルパスと切り出しサイズ（px）を表示する
   - 出力画像を Read ツールで表示する

## 典型的な連携フロー

```
1. /gui-test でスクリーンショット撮影 → tmp/capture.png
2. /annotate-grid tmp/capture.png --divide 8
   → tmp/annotated-capture.png で座標系を確認
3. 注目セルが (3,2) だと判明
4. /clip-image tmp/capture.png --grid-cell 3 2 8
   → tmp/clip-capture.png（注目領域のみ、OCR やテキスト検証に最適）
```

## 注意事項
- 出力先は `tmp/` を使う（`/tmp/` は使用禁止）
- `--grid-cell` の col/row は 0-origin かつ N 未満でなければならない
- `--rect` の座標は画像の左上原点（y は下向き増加）
