# 029: annotate-grid / clip-image ツール動作確認

## 目的

`tools/test-utils/annotate-grid` と `tools/test-utils/clip-image` が正しく動作することを確認する。
これらのツールは LLM が画像を判定する際の前処理として使用し、グリッド座標の確立と注目領域の切り出しを行う。

## 前提条件

- `tools/test-utils/build.sh` でビルド済みであること
- `tmp/` ディレクトリが存在すること（なければ `mkdir -p tmp`）
- テスト対象の PNG 画像が存在すること（SDIT スクリーンショットまたは任意の PNG）

---

## シナリオ A: annotate-grid --divide モード

### 手順

```bash
# 1. テスト画像準備（SDIT を起動してキャプチャ、または既存画像を使用）
./target/release/sdit &
SDIT_PID=$!
sleep 3
./tools/test-utils/capture-window sdit tmp/029-input.png
kill $SDIT_PID

# 2. annotate-grid --divide 4 実行
./tools/test-utils/annotate-grid tmp/029-input.png tmp/029-grid-divide.png --divide 4
echo "exit: $?"
```

### 期待結果

- exit code: 0
- `tmp/029-grid-divide.png` が生成される
- 標準出力に `Annotated: tmp/029-grid-divide.png (WxHpx)` が表示される
- 生成画像に 4×4 のグリッド線（赤半透明）が描画される
- 各セルの左上付近に `col,row` 形式（0-origin）のラベルが描画される
  - 左上セル: `0,0`、右上セル: `3,0`、左下セル: `0,3`、右下セル: `3,3`
- ラベルは黄色テキスト + 半透明黒背景

---

## シナリオ B: annotate-grid --every モード

### 手順

```bash
./tools/test-utils/annotate-grid tmp/029-input.png tmp/029-grid-every.png --every 100
echo "exit: $?"
```

### 期待結果

- exit code: 0
- `tmp/029-grid-every.png` が生成される
- 100px ごとの垂直線と水平線が描画される
- 垂直線の上端付近に `x=100`, `x=200`, ... のラベルが表示される
- 水平線の左端付近に `y=100`, `y=200`, ... のラベルが表示される

---

## シナリオ C: clip-image --grid-cell モード

### 手順

```bash
# annotate-grid --divide 4 と同じ分割でセル 1,1 を切り出す
./tools/test-utils/clip-image tmp/029-input.png tmp/029-clip-cell.png --grid-cell 1 1 4
echo "exit: $?"
```

### 期待結果

- exit code: 0
- `tmp/029-clip-cell.png` が生成される
- 標準出力に `Clipped: tmp/029-clip-cell.png (WxHpx)` が表示される
- クリップされた画像サイズは元画像の 1/4 程度（切り捨て誤差あり）
- 出力サイズの計算:
  - `cellWidth  = imgWidth / 4`
  - `cellHeight = imgHeight / 4`
  - セル 1,1: `x = cellWidth`, `y = cellHeight`

---

## シナリオ D: clip-image --rect モード

### 手順

```bash
# 左上 100x100 ピクセルを切り出す
./tools/test-utils/clip-image tmp/029-input.png tmp/029-clip-rect.png --rect 0 0 100 100
echo "exit: $?"
```

### 期待結果

- exit code: 0
- `tmp/029-clip-rect.png` が生成される
- 画像サイズが 100x100px であること

---

## シナリオ E: annotate-grid スタイルオプション

### 手順

```bash
# カスタム色（緑の線・白ラベル・青背景）
./tools/test-utils/annotate-grid tmp/029-input.png tmp/029-grid-custom.png \
    --divide 3 \
    --line-color 00FF00FF \
    --label-color FFFFFFFF \
    --label-bg 0000FF80 \
    --font-size 16
echo "exit: $?"
```

### 期待結果

- exit code: 0
- 緑色のグリッド線が描画される
- 白色のラベルテキスト + 半透明青背景で描画される
- 3×3 のグリッド（セル `0,0` 〜 `2,2`）が描画される

---

## シナリオ F: エッジケース — バリデーション

### 手順

```bash
# F-1: --divide 0 → exit 1
./tools/test-utils/annotate-grid tmp/029-input.png tmp/out.png --divide 0
echo "F-1 exit: $?"

# F-2: --every 0 → exit 1
./tools/test-utils/annotate-grid tmp/029-input.png tmp/out.png --every 0
echo "F-2 exit: $?"

# F-3: --grid-cell 範囲外（col >= N）→ exit 1
./tools/test-utils/clip-image tmp/029-input.png tmp/out.png --grid-cell 5 5 4
echo "F-3 exit: $?"

# F-4: --grid-cell 範囲外（row >= N）→ exit 1
./tools/test-utils/clip-image tmp/029-input.png tmp/out.png --grid-cell 2 4 4
echo "F-4 exit: $?"

# F-5: グリッドモード未指定 → exit 1
./tools/test-utils/annotate-grid tmp/029-input.png tmp/out.png
echo "F-5 exit: $?"

# F-6: クリップ領域未指定 → exit 1
./tools/test-utils/clip-image tmp/029-input.png tmp/out.png
echo "F-6 exit: $?"
```

### 期待結果

- F-1〜F-6 全て exit code: 1
- 各ケースで stderr にエラーメッセージが表示される
- `tmp/out.png` は生成されない（または無効ファイル）

---

## シナリオ G: 最終列・行の余りピクセル吸収

### 手順

```bash
# 割り切れないサイズでの確認（672px / 5 = 134.4px）
./tools/test-utils/annotate-grid tmp/029-input.png tmp/029-grid-5.png --divide 5
./tools/test-utils/clip-image tmp/029-input.png tmp/029-clip-last.png --grid-cell 4 4 5
echo "exit: $?"
```

### 期待結果

- exit code: 0
- 最終列（col=4）: `imgWidth - 4 * (imgWidth/5)` の幅
- 最終行（row=4）: `imgHeight - 4 * (imgHeight/5)` の高さ
- CGImage.cropping() が nil を返さずに成功すること

---

## 既知の注意事項

### annotate-grid の画像反転問題（2026-03-14 確認）

`screencapture` フォールバック経由で取得した PNG を annotate-grid に入力すると、
出力画像の**元画像コンテンツ**が上下反転して描画される。

- **影響**: グリッドラベル（0,0が左上）は正しい座標系
- **影響あり**: 元画像の視覚的な上下が反転するため、LLM が画像を判定する際に混乱を招く可能性がある
- **根本原因**: `screencapture` が生成する PNG の Y 軸方向と CGContext の flip 変換の不一致
- **回避策**: `capture-window` が ScreenCaptureKit モードで成功した場合は反転しない可能性がある（要確認）
- **Workaround**: 反転を前提にした上で座標を読む（ラベル自体は信頼できる）

---

## 実行結果サマリー（最終実行時に記録）

| ステップ | 内容 | 結果 |
|---|---|---|
| A | annotate-grid --divide 4 | - |
| B | annotate-grid --every 100 | - |
| C | clip-image --grid-cell | - |
| D | clip-image --rect | - |
| E | スタイルオプション | - |
| F | エッジケース | - |
| G | 余りピクセル吸収 | - |
