# Phase 24: 画像アノテーション・クリップツール

## 概要

LLM が画像を判定する際、全画面スクリーンショットを渡すと注目領域以外にトークンを浪費する。
グリッド座標を確立したうえで注目領域だけを切り出すことで、判定精度とトークン効率を同時に改善する。

**追加するツール:**
1. `annotate-grid` — 画像にグリッドと座標ラベルを描画
2. `clip-image` — 画像の指定領域を切り出す

---

## 1. CLI インターフェース設計

### `annotate-grid`

```
annotate-grid <input-png> <output-png> [options]

グリッドモード（いずれか1つ必須）:
  --divide N   縦横を N 等分（例: --divide 4 → 4×4 = 16 セル）
  --every N    N ピクセルごとに線を引く（例: --every 100）

スタイルオプション（任意）:
  --line-color  RRGGBBAA   グリッド線色（default: FF000080）
  --label-color RRGGBBAA   ラベル文字色（default: FFFF00FF）
  --label-bg    RRGGBBAA   ラベル背景色（default: 00000080）
  --font-size   N          ラベルフォントサイズ pt（default: 12）

Exit codes: 0=成功, 1=引数不正/読み込み失敗/書き出し失敗
```

**ラベル配置:**
- `--divide` モード: 各セル左上角に `col,row` 形式（0-origin）
- `--every` モード: X 軸の線上端に `x=N`、Y 軸の線左端に `y=N`

### `clip-image`

```
clip-image <input-png> <output-png> [options]

クリップ領域（いずれか1つ必須）:
  --rect x y width height    ピクセル座標で矩形指定
  --grid-cell col row N      annotate-grid --divide N と同じ分割で col,row を切り出す

Exit codes: 0=成功, 1=引数不正/読み込み失敗/書き出し失敗
```

---

## 2. Swift 実装方針

### `annotate-grid.swift`

使用 API:
- `CGDataProvider` + `CGImage(pngDataProviderSource:)` — 入力 PNG 読み込み
- `CGContext` — 描画バッファ（`CGImageAlphaInfo.premultipliedLast`）
- `ctx.draw(inputImage, in: fullRect)` — 元画像をコピー
- `ctx.setStrokeColor` + `ctx.strokeLineSegments(between:)` — グリッド線描画
- `CTFontCreateWithName` + `CTLineDraw` — CoreText ラベル描画
- `CGImageDestinationCreateWithURL` + `CGImageDestinationFinalize` — PNG 書き出し

CoreGraphics は左下原点のため Y 座標変換に注意。`render-text.swift` の
`flippedBaselineY = canvasHeight - padding - ascent` パターンを踏襲する。

### `clip-image.swift`

使用 API:
- `CGImage.cropping(to: CGRect)` — 矩形クリップ（`ocr-region.swift` に参照実装）
- `CGDataProvider` + `CGImage` — 読み込み
- `CGImageDestinationCreateWithURL` — 書き出し

`--grid-cell col row N` の計算:
```
cellWidth  = imageWidth / N     （最終列は余りを吸収）
cellHeight = imageHeight / N    （最終行は余りを吸収）
x          = col * cellWidth
y          = row * cellHeight
```

---

## 3. グリッドラベルの表示形式

| 属性 | 設計 |
|---|---|
| フォント | `Helvetica-Bold`、失敗時 UI フォントにフォールバック |
| フォントサイズ | デフォルト 12pt |
| 文字色 | デフォルト `#FFFF00FF`（不透明黄）。背景色に依存しにくい |
| 背景矩形 | デフォルト `#00000080`（半透明黒）。4px パディング付き |
| ラベル位置 | セル左上角から `(4px, 4px)` オフセット |
| グリッド線色 | デフォルト `#FF000080`（半透明赤）、線幅 1px |

---

## 4. `build.sh` への追記

```bash
echo "==> Building annotate-grid..."
swiftc "$SCRIPT_DIR/annotate-grid.swift" -o "$SCRIPT_DIR/annotate-grid" \
    -framework CoreGraphics -framework CoreText \
    -framework Foundation -framework ImageIO
echo "    OK: $SCRIPT_DIR/annotate-grid"

echo "==> Building clip-image..."
swiftc "$SCRIPT_DIR/clip-image.swift" -o "$SCRIPT_DIR/clip-image" \
    -framework CoreGraphics -framework Foundation -framework ImageIO
echo "    OK: $SCRIPT_DIR/clip-image"
```

---

## 5. スキル設計

### `/annotate-grid` スキル

1. `tools/test-utils/annotate-grid` がビルド済みか確認（未なら `build.sh` 実行）
2. 出力パスを未指定なら `tmp/annotated-<入力ファイル名>` に自動設定
3. コマンド実行
4. 出力画像を `Read` ツールで表示
5. `--grid-cell col row N` や `--rect` でクリップできると案内

### `/clip-image` スキル

1. ビルド確認
2. 出力パスを未指定なら `tmp/clip-<入力ファイル名>` に自動設定
3. コマンド実行
4. 出力画像を `Read` ツールで表示し、切り出しサイズを報告

**典型的な連携フロー:**
```
1. /gui-test でスクリーンショット撮影 → tmp/capture.png
2. /annotate-grid tmp/capture.png --divide 8
   → tmp/annotated-capture.png で座標系を確認
3. 注目セルが (3,2) だと判明
4. /clip-image tmp/capture.png --grid-cell 3 2 8
   → tmp/clip-capture.png（注目領域のみ）
5. /verify-text tmp/clip-capture.png "期待テキスト"
```

---

## 6. セキュリティ考慮

- 入力・出力パス両方に `capture-window.swift` L50-58 のパストラバーサル防止を適用
- `--divide 0` / `--every 0`: 引数バリデーションで `>= 1` を強制
- `--grid-cell col row N` で `col >= N` / `row >= N`: 引数バリデーションで弾く
- `CGImage.cropping(to:)` が `nil` を返した場合（範囲外）: エラーメッセージで `exit(1)`

---

## 7. 実装参照先

| 要素 | 参照ファイル |
|---|---|
| パストラバーサル防止 | `capture-window.swift` L50-58 |
| PNG 読み込み | `ocr-region.swift` L34-39 |
| CGContext 作成・描画 | `render-text.swift` L211-225 |
| PNG 書き出し | `render-text.swift` L321-339 |
| CoreText ラベル描画 | `render-text.swift` L238-253 |
| 引数パーサー構造 | `verify-text.swift` L43-98 |
| 矩形クロップ | `ocr-region.swift` L51-58 |

---

## 8. 完了条件

- [ ] `tools/test-utils/annotate-grid.swift` 実装・ビルド成功
- [ ] `tools/test-utils/clip-image.swift` 実装・ビルド成功
- [ ] `tools/test-utils/build.sh` に両ツールのビルドステップ追記
- [ ] `tools/test-utils/README.md` にツール一覧・使い方追記
- [ ] `.claude/skills/annotate-grid.md` 作成
- [ ] `.claude/skills/clip-image.md` 作成
- [ ] パストラバーサル防止・引数バリデーション実装済み
- [ ] 実際のスクリーンショットで動作確認

## セキュリティレビュー

セキュリティ観点は §6 に記載済み。実装後にレビューを実施する。
