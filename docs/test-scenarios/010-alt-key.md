# 010: Alt+key 送信確認

## 目的
Alt+key が ESC プレフィックス付きで PTY に送信され、vim/emacs 等のアプリケーションで正しく動作することを確認する。

## 前提条件
- `cargo build --package sdit`
- `tools/test-utils/build.sh`
- Screen Recording 権限 + OS 再起動
- Accessibility 権限

## 手順
1. SDIT をバックグラウンドで起動する
2. window-info でウィンドウの存在を確認する（最大 15 秒ポーリング）
3. send-keys で `cat -v` を入力する（制御文字を可視化するモードで起動）
4. send-keys で Return キーを送信する
5. 1 秒待機する（cat の起動を待つ）
6. send-keys で Alt+a を送信する
7. 2 秒待機する（PTY 出力 → 描画の伝搬を待つ）
8. capture-window でスクリーンショットを撮る（`tmp/010-alt-key.png`）
9. send-keys で Ctrl+C を送信して cat を終了する

## 期待結果
- ウィンドウが表示されている（window-info が exit 0）
- スクリーンショットに `^[a`（ESC + a）が表示されている
- （将来）AI 視覚分析でスクリーンショットに `^[a` が描画されていることを確認

## クリーンアップ
- SDIT プロセスを終了する
- `tmp/010-alt-key.png` を削除する

## 関連
- Phase 5.5: Alt → ESC プレフィックス変換の実装
- `crates/sdit/src/input.rs` の Alt キー処理
