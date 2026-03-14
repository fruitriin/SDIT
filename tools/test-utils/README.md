# tools/test-utils — GUI テストユーティリティ

macOS 向けの GUI テスト補助ツール群。
SDIT の `crates/sdit/tests/gui_interaction.rs` から呼び出される。

## ツール一覧

| ツール | 言語 | 用途 |
|---|---|---|
| `window-info` | Swift | AXUIElement でウィンドウ属性を JSON 出力 |
| `capture-window` | Swift | ScreenCaptureKit でウィンドウを PNG キャプチャ |
| `render-text` | Swift | CoreText で対照群テキスト画像を生成 |
| `verify-text` | Swift | OCR + 輝度 + SSIM でテキスト描画品質を検証 |
| `annotate-grid` | Swift | PNG にグリッド線・座標ラベルを描画 |
| `clip-image` | Swift | PNG の指定領域（矩形・グリッドセル）を切り出す |
| `send-keys.sh` | bash | osascript でキーストロークを送信 |
| `build.sh` | bash | Swift スクリプトをコンパイル |

---

## ビルド手順

```bash
cd tools/test-utils
./build.sh
```

`window-info` と `capture-window` のバイナリが生成される。

### 必要な環境

- macOS 15 (Sequoia) 以降（ScreenCaptureKit の `SCScreenshotManager` API）
- Xcode Command Line Tools（`swiftc` が使えること）

```bash
xcode-select --install
```

---

## 権限設定

### Screen Recording（`capture-window` に必要）

1. **System Settings → Privacy & Security → Screen Recording** を開く
2. リストに `capture-window` が表示されていなければ、`+` ボタンで追加する
3. トグルをオンにする
4. **OS を再起動する**（再起動しないと権限が有効にならない）

> **注意**: 権限付与後は必ず再起動してください。再起動なしでは `SCStreamError` が発生します。

### 権限のリセット（テスト環境のクリーンアップ用）

```bash
# capture-window の Screen Recording 権限をリセット
tccutil reset ScreenCapture

# その後、再度 System Settings から権限を付与 → 再起動
```

### Accessibility（`send-keys.sh` に必要）

1. **System Settings → Privacy & Security → Accessibility** を開く
2. `send-keys.sh` を実行するターミナルアプリを許可する
   （例: Terminal.app、iTerm2、Ghostty など）

---

## 各ツールの使い方

### window-info

```bash
# ビルド後のバイナリを使用
./window-info sdit

# 出力例:
# {
#   "focused" : true,
#   "pid" : 12345,
#   "position" : { "x" : 100, "y" : 200 },
#   "size" : { "height" : 600, "width" : 800 },
#   "title" : "sdit"
# }
```

- ウィンドウが見つからない場合は `exit 1`
- 権限エラーはなし（AXUIElement は Accessibility 不要で読み取り可能）

### capture-window

```bash
# ビルド後のバイナリを使用
# 出力先はプロジェクトルートの tmp/ を使う
./capture-window sdit ../../tmp/test-capture.png

# 成功時の出力:
# Captured: ../../tmp/test-capture.png (1600x1200px)
```

- Screen Recording 権限がない場合は `exit 2`
- ウィンドウが見つからない場合は `exit 1`
- 出力先ディレクトリが存在しない場合は自動作成

### send-keys.sh

```bash
./send-keys.sh sdit "hello world"
# → sdit プロセスにキーストロークを送信

./send-keys.sh sdit "echo test"
# → Enter は含まれない（必要なら \n を追加して改変）
```

- プロセスが見つからない場合は `exit 2`
- Accessibility 権限がない場合は `exit 2`

### annotate-grid

```bash
# 4×4 グリッドを描画（各セルに "col,row" ラベル）
./annotate-grid ../../tmp/capture.png ../../tmp/annotated.png --divide 4

# 100px ごとに線を引く
./annotate-grid ../../tmp/capture.png ../../tmp/annotated.png --every 100

# スタイル指定
./annotate-grid ../../tmp/capture.png ../../tmp/annotated.png \
    --divide 8 --font-size 20 --line-color 0000FF80
```

- `--divide N` モード: 各セル左上に `col,row`（0-origin）を描画
- `--every N` モード: 各垂直線上端に `x=N`、各水平線左端に `y=N` を描画
- `--divide 0` / `--every 0` は `exit 1`

### clip-image

```bash
# annotate-grid --divide 4 で確認したセル (1,1) を切り出す
./clip-image ../../tmp/capture.png ../../tmp/clip.png --grid-cell 1 1 4

# ピクセル座標で切り出す
./clip-image ../../tmp/capture.png ../../tmp/clip.png --rect 100 200 400 300

# 成功時の出力:
# Clipped: ../../tmp/clip.png (400x300px)
```

- `col >= N` / `row >= N` は `exit 1`
- cropping が失敗した場合（範囲外等）は `exit 1`
- 出力先ディレクトリが存在しない場合は自動作成

### 典型的な連携フロー

```bash
# 1. スクリーンショット撮影
./capture-window sdit ../../tmp/capture.png

# 2. グリッドを描画して座標系を確認
./annotate-grid ../../tmp/capture.png ../../tmp/annotated.png --divide 8

# 3. 注目セル (3,2) を切り出す
./clip-image ../../tmp/capture.png ../../tmp/clip.png --grid-cell 3 2 8

# 4. OCR で検証
./verify-text ../../tmp/clip.png "期待テキスト"
```

---

## GUI テストの実行

```bash
# プロジェクトルートから
cargo test --test gui_interaction -- --ignored

# 特定のテストのみ
cargo test --test gui_interaction window_appears -- --ignored
```

`#[ignore]` 属性付きテストのため、`-- --ignored` フラグが必要。

---

## トラブルシューティング

| 症状 | 原因 | 対処 |
|---|---|---|
| `SCStreamError` / exit 2 | Screen Recording 権限なし | 権限付与 → 再起動 |
| `osascript` が失敗 | Accessibility 権限なし | ターミナルを Accessibility に追加 |
| バイナリが見つからない | ビルド未実施 | `./build.sh` を実行 |
| ウィンドウが見つからない | sdit が起動していない | sdit を先に起動する |
