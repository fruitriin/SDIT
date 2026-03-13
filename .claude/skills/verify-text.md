---
name: verify-text
description: スクリーンショットのテキスト描画品質を自動検証する（OCR + 輝度分析 + SSIM）。画像を読まずテキストレポートで判定。
user_invocable: true
---

# テキスト描画の自動検証

## 引数
- `$ARGUMENTS`: `<画像パス> <期待テキスト> [options]`
  - 例: `tmp/capture.png "こんにちは世界"`
  - 例: `tmp/capture.png "HELLO" --reference tmp/ref.png --cells tmp/cells.json`
  - 省略時: 使い方を表示する

## 手順

### 引数なしの場合
以下の使い方を表示する:

```
verify-text <image> <expected-text> [options]

Options:
  --region x,y,w,h       検査領域（画像ピクセル座標）
  --reference <path>      対照群画像（SSIM 比較用、/render-text で生成）
  --cells <json-file>     セル境界 JSON（/render-text の出力）
  --edge-margin <px>      右端検査マージン (default: 6)
  --ssim-threshold <f>    SSIM 閾値 (default: 0.3)

検証レイヤー:
  [OCR]       Vision.framework でテキスト認識 → 期待値と比較
  [LUMINANCE] セル内インク有無 + 右端クリッピング検出
  [SSIM]      対照群とのセル単位構造類似度

Exit: 0=全PASS, 1=エラー, 3=いずれかFAIL
```

### 引数ありの場合

1. `tools/test-utils/verify-text` がビルド済みか確認する
   - 未ビルドなら `tools/test-utils/build.sh` を実行する

2. 指定された画像ファイルが存在するか確認する
   - 存在しない場合、`tmp/` 内の最新の PNG を候補として提示する

3. 検証モードを判定する:

   **パターン A: ASCII 存在確認**（--cells/--reference なし）
   ```bash
   ./tools/test-utils/verify-text <image> <text>
   ```
   OCR のみ。最小コスト。

   **パターン B: CJK 3層フル検証**（--cells + --reference あり）
   ```bash
   ./tools/test-utils/verify-text <image> <text> --cells <json> --reference <ref>
   ```
   OCR + 輝度 + SSIM。

   **パターン C: 自動対照群生成 + 3層フル検証**（--auto-reference）
   ユーザーが `--auto-reference` または `--auto` を指定した場合:
   ```bash
   # 1. render-text で対照群を自動生成
   ./tools/test-utils/render-text --mono --cell-info <text> tmp/verify-auto-ref.png \
       | tail -n +2 > tmp/verify-auto-cells.json

   # 2. 3層検証
   ./tools/test-utils/verify-text <image> <text> \
       --cells tmp/verify-auto-cells.json \
       --reference tmp/verify-auto-ref.png
   ```

4. コマンドを実行する

5. 結果を報告する:
   - レポート全文をそのまま表示する（テキストのみ、画像読み込み不要）
   - FAIL の場合は失敗したチェックを強調する
   - FAIL 時のみ、画像を Read ツールで表示して視覚確認を提案する

## 注意事項
- 出力先は `tmp/` を使う（`/tmp/` は使用禁止）
- `--auto-reference` 使用時の一時ファイル（`tmp/verify-auto-*`）は検証後も保持する（デバッグ用）
- OCR は絵文字・リガチャの認識精度が低い場合がある → SSIM スコアで補完判定する
- SSIM FAIL + OCR PASS の場合: フォントの差異は許容範囲内の可能性がある（閾値調整を検討）
