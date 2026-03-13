# 001: 基本的な echo 動作確認

## 目的
SDIT の GUI ウィンドウでキー入力が PTY に送信され、echo の結果が画面に描画されることを確認する。

## 前提条件
- `cargo build --package sdit`
- `tools/test-utils/build.sh`
- Screen Recording 権限 + OS 再起動
- Accessibility 権限

## 手順
1. SDIT をバックグラウンドで起動する
2. window-info でウィンドウの存在を確認する（最大 15 秒ポーリング）
3. send-keys で `echo SDIT_ECHO_TEST` を入力する
4. send-keys で Return キーを送信する
5. 2 秒待機する（PTY 出力 → 描画の伝搬を待つ）
6. capture-window でスクリーンショットを撮る（`tmp/001-echo.png`）

## 自動検証（verify-text）

```bash
# キャプチャ後に実行
./tools/test-utils/verify-text tmp/001-echo.png "SDIT_ECHO_TEST"
# OCR で表示テキストを自動照合。exit 0 = PASS
```

- `--cells` / `--reference` は不要（ASCII テキストの存在確認のみ）
- OCR PASS なら「画面に文字が描画されている」ことが確定する

## 期待結果
- ウィンドウが表示されている（window-info が exit 0）
- スクリーンショットのファイルサイズが 10 KiB 以上（空白でない）
- **verify-text が exit 0**（OCR で "SDIT_ECHO_TEST" が認識される）

## クリーンアップ
- SDIT プロセスを終了する
- `tmp/001-echo.png` を削除する

## 関連
- Feedback: "gui上で文字が入力できない気がする"
- `crates/sdit/src/main.rs` の `key_to_bytes()` 関数
