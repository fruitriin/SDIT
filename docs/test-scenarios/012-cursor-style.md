# 012: カーソルスタイル変更（DECSCUSR）

## 目的
DECSCUSR エスケープシーケンスでカーソル形状（ブロック/アンダーライン/バー）が変更されること、
および DECSCUSR 0 で設定ファイルのデフォルト値に復帰することを確認する。

## 前提条件
- `cargo build --package sdit`
- `tools/test-utils/build.sh`
- Screen Recording 権限 + OS 再起動
- Accessibility 権限

## 手順
1. SDIT をバックグラウンドで起動する
2. window-info でウィンドウの存在を確認する（最大 15 秒ポーリング）
3. capture-window でデフォルトのカーソル状態を撮影する（`tmp/012-cursor-default.png`）
4. send-keys で `printf '\033[5 q'` を入力する（点滅バーカーソルへ変更）
5. send-keys で Return キーを送信する
6. 1 秒待機する（カーソル形状の反映を待つ）
7. capture-window で変更後のカーソル状態を撮影する（`tmp/012-cursor-bar.png`）
8. send-keys で `printf '\033[1 q'` を入力する（点滅ブロックカーソルへ変更）
9. send-keys で Return キーを送信する
10. 1 秒待機する（カーソル形状の反映を待つ）
11. capture-window で変更後のカーソル状態を撮影する（`tmp/012-cursor-block.png`）
12. send-keys で `printf '\033[0 q'` を入力する（DECSCUSR 0: デフォルトに復帰）
13. send-keys で Return キーを送信する
14. 1 秒待機する
15. capture-window でデフォルト復帰後のカーソル状態を撮影する（`tmp/012-cursor-reset.png`）

## 期待結果
- ウィンドウが表示されている（window-info が exit 0）
- 4 枚のスクリーンショットが生成されており、それぞれファイルサイズが 10 KiB 以上（空白でない）
- `012-cursor-reset.png` は `012-cursor-default.png` と同じカーソル形状（デフォルト = Block）
- （将来）AI 視覚分析で 3 枚（default/bar/block）のカーソル形状が互いに異なることを確認

## 設定ファイルによるデフォルト変更（手動確認用）
```toml
[cursor]
style = "bar"
blinking = false
color = "#ff6600"
```
上記設定で SDIT を起動すると、初期カーソルが Bar になり、DECSCUSR 0 で Bar に戻ることを確認する。

## クリーンアップ
- SDIT プロセスを終了する
- `tmp/012-cursor-default.png` を削除する
- `tmp/012-cursor-bar.png` を削除する
- `tmp/012-cursor-block.png` を削除する
- `tmp/012-cursor-reset.png` を削除する

## 関連
- Phase 5.5: DECSCUSR カーソルスタイル変更の実装
- Phase 14.1: カーソル設定（CursorConfig）+ DECSCUSR 0 デフォルト復帰
- `crates/sdit-core/src/terminal/handler.rs` の DECSCUSR 処理
- `crates/sdit-core/src/render/pipeline.rs` のカーソル描画
- `crates/sdit-core/src/config/mod.rs` の CursorConfig
