# 009: 日本語 CJK 文字の描画確認

## 目的
日本語文字が豆腐（□）にならず正しく描画され、全角文字が 2 セル幅で表示されることを確認する。
図形記号（口 □ ■ ● ▲）も含めて検証することで、フォント未搭載時の豆腐化を見分けやすくする。

### テスト文字列の選定理由

| 文字 | 種別 | 選定理由 |
|---|---|---|
| こんにちわ世界 | 日本語ひらがな・漢字 | 基本的な CJK 描画 |
| 口 | CJK 漢字（U+53E3） | □ との形状比較。正常描画なら縦画がある |
| □ | 白四角（U+25A1） | 豆腐グリフと同形状。正常描画なら細い枠線のみ |
| ■ | 黒四角（U+25A0） | 塗りつぶし。輝度が高く存在確認しやすい |
| ● | 黒丸（U+25CF） | 丸形。■ との比較で形状描画を確認 |
| ▲ | 黒三角（U+25B2） | 三角。幾何学図形のグリフ描画を確認 |

## 前提条件
- `cargo build --package sdit`
- `tools/test-utils/build.sh`
- Screen Recording 権限 + OS 再起動
- Accessibility 権限

## 手順
1. SDIT をバックグラウンドで起動する
2. window-info でウィンドウの存在を確認する（最大 15 秒ポーリング）
3. send-keys で `echo こんにちは世界` を **スペース込みで1文字列として** 入力する
   ```bash
   ./tools/test-utils/send-keys.sh sdit "echo こんにちは世界"
   # ⚠️ "echo" と "こんにちは世界" を別々に送ると "echoこんにちは世界" になる
   ```
   IME 干渉が起きる場合は PTY 直接書き込みを使う:
   ```bash
   printf "echo こんにちは世界\r" > /dev/ttys$(./tools/test-utils/window-info sdit | python3 -c "import json,sys; print(json.load(sys.stdin)['tty'].replace('/dev/tty',''))" 2>/dev/null || echo "NNN")
   ```
4. send-keys を使った場合は Return キーを送信する（PTY 直接書き込みの場合は不要）
5. 2 秒待機する（PTY 出力 → 描画の伝搬を待つ）
6. capture-window でスクリーンショットを撮る（`tmp/009-cjk.png`）

## 自動検証（verify-text + render-text）

### 検証 A: ひらがな・漢字

```bash
./tools/test-utils/render-text --mono --cell-info "こんにちわ世界" tmp/009-ref.png \
    | tail -n +2 > tmp/009-cells.json
./tools/test-utils/verify-text tmp/009-cjk.png "こんにちわ世界" \
    --cells tmp/009-cells.json --reference tmp/009-ref.png
```

### 検証 B: 図形記号（annotate-grid で拡大して目視確認）

```bash
# 図形記号を echo した後のスクリーンショット
./tools/test-utils/capture-window sdit tmp/009-shapes.png

# グリッドで拡大確認
./tools/test-utils/annotate-grid tmp/009-shapes.png tmp/009-shapes-grid.png --divide 8

# 対象セルを clip-image で切り出し
./tools/test-utils/clip-image tmp/009-shapes.png tmp/009-shapes-clip.png --grid-cell 0 1 8
```

**期待する見え方:**
- `口` → 縦画・横画がある四角（□ より線が多い）
- `□` → 細い枠線のみの白四角（これが豆腐と同形なので区別のポイント）
- `■` → 全面塗りつぶしの黒四角（輝度が高い）
- `●` → 全面塗りつぶしの黒丸
- `▲` → 三角形

**判定基準:**
- `口` と `□` の形状が異なって見える → CJK フォントが正しく読み込まれている
- `■ ● ▲` に十分な輝度がある → 記号グリフが描画されている

## 期待結果
- ウィンドウが表示されている（window-info が exit 0）
- スクリーンショットのファイルサイズが 10 KiB 以上（空白でない）
- **verify-text が exit 0**（OCR + 輝度 + SSIM 全 PASS）

## クリーンアップ
- SDIT プロセスを終了する
- `tmp/009-cjk.png` `tmp/009-ref.png` `tmp/009-cells.json` を削除する

## 関連
- Phase 5.3: 日本語フォント対応
- `crates/sdit-core/src/font/`
- `crates/sdit-core/src/grid/`
