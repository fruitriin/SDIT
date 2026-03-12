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
3. send-keys で `echo こんにちは世界` を入力する
4. send-keys で Return キーを送信する
5. 2 秒待機する（PTY 出力 → 描画の伝搬を待つ）
6. capture-window でスクリーンショットを撮る（`tmp/009-cjk.png`）

## 期待結果
- ウィンドウが表示されている（window-info が exit 0）
- スクリーンショットのファイルサイズが 10 KiB 以上（空白でない）
- （将来）AI 視覚分析で以下を検証する:
  - 「こんにちは世界」が豆腐（□）にならず実際の文字として描画されていること
  - 全角文字が半角文字の 2 倍の幅で描画されており、文字同士が重なっていないこと

## クリーンアップ
- SDIT プロセスを終了する
- `tmp/009-cjk.png` を削除する

## 関連
- Phase 5.3: 日本語フォント対応
- `crates/sdit-core/src/font/`
- `crates/sdit-core/src/grid/`
