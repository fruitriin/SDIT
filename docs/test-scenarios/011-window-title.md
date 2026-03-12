# 011: ウィンドウタイトル更新（OSC 0/2）

## 目的
OSC 0/2 エスケープシーケンスでウィンドウタイトルが更新されることを確認する。

## 前提条件
- `cargo build --package sdit`
- `tools/test-utils/build.sh`
- Screen Recording 権限 + OS 再起動
- Accessibility 権限

## 手順
1. SDIT をバックグラウンドで起動する
2. window-info でウィンドウの存在を確認し、初期タイトルを記録する（最大 15 秒ポーリング）
3. send-keys で `printf '\033]0;SDIT_TITLE_TEST\007'` を入力する
4. send-keys で Return キーを送信する
5. 2 秒待機する（ウィンドウタイトルの反映を待つ）
6. window-info でウィンドウタイトルを再取得し、変更されたことを確認する

## 期待結果
- ウィンドウが表示されている（window-info が exit 0）
- 手順 6 で取得したウィンドウタイトルが `SDIT_TITLE_TEST` に変わっている
- 初期タイトルと手順 6 のタイトルが異なる

## クリーンアップ
- SDIT プロセスを終了する

## 関連
- Phase 5.5: OSC 0/2 ウィンドウタイトル更新の実装
- `crates/sdit-core/src/terminal/handler.rs` の OSC 処理
