# 008: TOML 設定によるフォント・カラーテーマ反映確認

## 目的
`sdit.toml` でフォントとカラーテーマを指定したとき、起動時にその設定が正しく反映されることを確認する。

## 前提条件
- `cargo build --package sdit`
- `tools/test-utils/build.sh`
- Screen Recording 権限 + OS 再起動
- Accessibility 権限

## 手順
1. テスト用設定ファイルを作成する（`tmp/test-config.toml`）
   - フォント: `Monaco` / サイズ: `18`
   - カラーテーマ: `gruvbox-dark`
2. 環境変数またはコマンドライン引数で `tmp/test-config.toml` のパスを指定し、SDIT をバックグラウンドで起動する
3. window-info でウィンドウの存在を確認する（最大 15 秒ポーリング）
4. send-keys で `echo CONFIG_TEST` を入力する
5. send-keys で Return キーを送信する
6. 2 秒待機する（PTY 出力 → 描画の伝搬を待つ）
7. capture-window でスクリーンショットを撮る（`tmp/008-config.png`）
8. SDIT を終了し、設定ファイルを指定せずにデフォルト設定（Menlo 14pt, catppuccin-mocha）で再起動する
9. window-info でウィンドウの存在を確認する（最大 15 秒ポーリング）
10. capture-window でスクリーンショットを撮る（`tmp/008-default.png`）

## 期待結果
- どちらの起動でもウィンドウが表示されている（window-info が exit 0）
- `tmp/008-config.png` と `tmp/008-default.png` のファイルサイズがともに 10 KiB 以上（空白でない）
- 2 枚のスクリーンショットを目視比較したとき、フォントサイズ・配色に明確な違いが確認できる
- （将来）AI 視覚分析で、`008-config.png` が gruvbox-dark 配色（黄褐色背景系）であり、`008-default.png` が catppuccin-mocha 配色（暗青色背景系）であることを検証する

## クリーンアップ
- SDIT プロセスをすべて終了する
- `tmp/test-config.toml` を削除する
- `tmp/008-config.png` を削除する
- `tmp/008-default.png` を削除する

## 関連
- Phase 5: `docs/plans/phase5-config-polish.md`
- `crates/sdit-core/src/config/`
