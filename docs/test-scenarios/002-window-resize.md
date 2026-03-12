# 002: ウィンドウリサイズ時のグリッド再計算確認

## 目的
ウィンドウをリサイズしたとき、グリッドサイズが再計算され、PTY に SIGWINCH が通知されることを確認する。

## 前提条件
- `cargo build --package sdit`
- `tools/test-utils/build.sh`
- Screen Recording 権限 + OS 再起動
- Accessibility 権限

## 手順
1. SDIT をバックグラウンドで起動する
2. window-info でウィンドウの存在と初期サイズを確認する（最大 15 秒ポーリング）
3. AppleScript（`osascript`）でウィンドウを別のサイズにリサイズする（例: 幅 1200px → 800px）
4. 1 秒待機する（SIGWINCH 伝搬 → PTY 側への通知を待つ）
5. send-keys で `tput cols; tput lines` を入力する
6. send-keys で Return キーを送信する
7. 2 秒待機する（PTY 出力 → 描画の伝搬を待つ）
8. capture-window でスクリーンショットを撮る（`tmp/002-resize.png`）

## 期待結果
- ウィンドウが表示されている（window-info が exit 0）
- `tput cols` と `tput lines` の出力値がリサイズ後のウィンドウサイズから計算されるグリッドサイズと一致する
- スクリーンショットのファイルサイズが 10 KiB 以上（空白でない）
- （将来）AI 視覚分析でスクリーンショット上のグリッド描画がリサイズ後のサイズで正しく更新されていることを確認

## クリーンアップ
- SDIT プロセスを終了する
- `tmp/002-resize.png` を削除する

## 関連
- Phase 2: 最初の SDI ウィンドウ
- `crates/sdit/src/render.rs` のグリッドサイズ再計算処理
- `crates/sdit-core/src/pty/` の SIGWINCH 送信処理
