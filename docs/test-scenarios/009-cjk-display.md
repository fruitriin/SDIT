# 009: 日本語 CJK 文字の描画確認

## 目的
日本語文字が豆腐（□）にならず正しく描画され、全角文字が 2 セル幅で表示されることを確認する。

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

```bash
# 1. 対照群画像 + セル境界 JSON を生成
./tools/test-utils/render-text --mono --cell-info "こんにちは世界" tmp/009-ref.png \
    | tail -n +2 > tmp/009-cells.json

# 2. 3層一括検証
./tools/test-utils/verify-text tmp/009-cjk.png "こんにちは世界" \
    --cells tmp/009-cells.json \
    --reference tmp/009-ref.png
# exit 0 = 全チェック PASS
```

**検証内容:**
- **OCR 照合**: 「こんにちは世界」が豆腐（□）にならず実際の文字として認識される
- **輝度分析**: 各文字セルにインクが存在し、右端クリッピングがない
- **SSIM 比較**: CoreText の正解レンダリングと構造が類似している

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
