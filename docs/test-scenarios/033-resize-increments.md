# 033: ウィンドウリサイズのセル整数倍スナップ確認

## 目的
`[window] resize_increments = true` 設定時、ウィンドウリサイズがセルサイズの整数倍にスナップされることを確認する。

## 前提条件
- `cargo build --package sdit`
- `tools/test-utils/build.sh`
- Screen Recording 権限 + Accessibility 権限

## 手順

### A: resize_increments = true

1. 設定ファイルに `[window]` セクションで `resize_increments = true` を追加する
2. SDIT をバックグラウンドで起動する
3. window-info でウィンドウの存在と初期サイズを確認する（最大 15 秒ポーリング）
4. AppleScript でウィンドウを半端なサイズにリサイズする（例: 幅 807px、高さ 503px）
5. 1 秒待機する
6. window-info で実際のウィンドウサイズを取得する
7. send-keys で `tput cols; tput lines` + Return を入力する
8. 2 秒待機後、capture-window でスクリーンショットを撮る（`tmp/033-resize-snap.png`）
9. SDIT を終了する

### B: resize_increments = false（デフォルト）

1. 設定ファイルで `resize_increments = false` にする（またはキーを削除する）
2. SDIT をバックグラウンドで起動する
3. window-info で初期サイズを確認する
4. AppleScript で同じ半端なサイズにリサイズする
5. 1 秒待機後、window-info で実際のサイズを取得する
6. SDIT を終了する

### C: フォントサイズ変更後のインクリメント更新

1. `resize_increments = true` で SDIT を起動する
2. Cmd+= でフォントサイズを拡大する
3. 1 秒待機後、AppleScript で半端なサイズにリサイズする
4. window-info で実際のサイズを確認する
5. SDIT を終了する

## 期待結果

### A（true）
- リサイズ後のウィンドウサイズがセルサイズの整数倍 + パディング分に一致する（macOS が `resizeIncrements` を尊重してスナップする）
- `tput cols` / `tput lines` の出力がウィンドウサイズと整合する
- スクリーンショットでグリッド端に半端な余白がない

### B（false）
- リサイズ後のウィンドウサイズが指定した値とほぼ一致する（スナップされない）

### C（フォント変更後）
- フォントサイズ変更後もリサイズインクリメントが正しく更新され、新しいセルサイズの整数倍にスナップされる

## クリーンアップ
- SDIT プロセスを終了する
- `tmp/033-*.png` を削除する
- テスト用設定ファイルを元に戻す

## 関連
- Phase 25.1: `docs/plans/phase25.1-resize-increments.md`
- `crates/sdit/src/window_ops.rs` — リサイズインクリメント設定
- `crates/sdit/src/event_loop.rs` — フォントサイズ変更時のインクリメント更新
- 002-window-resize — 基本リサイズシナリオ
