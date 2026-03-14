# gui-test: annotate-grid / clip-image ツール知見

## 概要

Phase 24 で追加した `annotate-grid` と `clip-image` の動作確認で得た知見。
LLM による画像判定の前処理として使用するツール群。

## ビルド

`tools/test-utils/build.sh` に以下が追加済み:

```bash
swiftc annotate-grid.swift -o annotate-grid \
    -framework CoreGraphics -framework CoreText -framework Foundation -framework ImageIO
swiftc clip-image.swift -o clip-image \
    -framework CoreGraphics -framework Foundation -framework ImageIO
```

## 動作確認済みの基本フロー

```bash
# 1. SDIT キャプチャ
./tools/test-utils/capture-window sdit tmp/capture.png

# 2. グリッドアノテーション（4×4分割）
./tools/test-utils/annotate-grid tmp/capture.png tmp/annotated.png --divide 4
# → 出力: "Annotated: tmp/annotated.png (672x436px)"

# 3. 注目セルを切り出し
./tools/test-utils/clip-image tmp/capture.png tmp/clip.png --grid-cell 1 1 4
# → 出力: "Clipped: tmp/clip.png (168x109px)"

# 4. --every モードでピクセル座標確認
./tools/test-utils/annotate-grid tmp/capture.png tmp/every.png --every 100
# → x=100〜, y=100〜 のラベルが表示される
```

## バリデーション挙動

| コマンド | exit code | エラーメッセージ |
|---|---|---|
| `--divide 0` | 1 | `--divide は 1 以上が必要です` |
| `--every 0` | 1 | `--every は 1 以上が必要です` |
| `--grid-cell 5 5 4` | 1 | `col (5) は N (4) 未満でなければなりません` |
| `--grid-cell 2 4 4` | 1 | `row (4) は N (4) 未満でなければなりません` |

## 既知のバグ: annotate-grid 画像反転問題

`screencapture` フォールバック経由で取得した PNG を annotate-grid に入力すると、
**元画像コンテンツが上下反転**して出力される。

**症状:**
- SDIT のタイトルバーが画像下部に表示される
- ターミナルコンテンツが上下逆さまになる

**ラベルは正しい:**
- `0,0` が左上、`N-1,N-1` が右下（正しい座標系）

**根本原因:**
- `screencapture` が生成する PNG は既に正立（左上原点、Y 下向き）
- `annotate-grid.swift` L350-352 の flip 変換（translate + scale(-1)）が余分に適用される

**修正方針（未実施）:**
- CGImage の向きを確認して条件分岐する
- または flip 変換を削除して CGContext の Y 軸座標変換を drawLabel 側で正しく処理する

**Workaround:**
- 反転を前提にして使用する（ラベル座標は信頼できる）
- ScreenCaptureKit モードが成功する環境では反転しない可能性がある（未確認）

## ラベルの座標系

`--divide N` モードでのラベル配置:
- 各セルの左上角から `(4px, 4px)` オフセット
- フォーマット: `col,row`（0-origin）
- デフォルト色: 黄色テキスト + 半透明黒背景

`--every N` モードのラベル配置:
- 垂直線の上端: `x=N`
- 水平線の左端: `y=N`

## clip-image の最終列・行処理

`--grid-cell col row N` での切り出しサイズ:
```
cellW = imgW / n        （整数除算）
cellH = imgH / n        （整数除算）
最終列: clipW = imgW - col * cellW   （余りピクセルを吸収）
最終行: clipH = imgH - row * cellH   （余りピクセルを吸収）
```

## 典型的な連携フロー

```
1. capture-window → tmp/capture.png
2. annotate-grid --divide 8 → tmp/annotated.png（座標系確認）
3. 注目セルが (3,2) だと判明
4. clip-image --grid-cell 3 2 8 → tmp/clip.png（注目領域のみ）
5. verify-text tmp/clip.png "期待テキスト"
```
