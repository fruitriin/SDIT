# 006: セッション切り替え

## 目的
Ctrl+Tab でセッションを切り替えると、端末グリッドに表示される内容が切り替え先のセッションの内容に変わることを確認する。

## 前提条件
- `cargo build --package sdit`
- `tools/test-utils/build.sh`
- Screen Recording 権限 + OS 再起動
- Accessibility 権限

## 手順
1. SDIT をバックグラウンドで起動する
2. window-info でウィンドウの存在を確認する（最大 15 秒ポーリング）
3. send-keys で `echo SESSION_ONE` を入力し、Return キーを送信する
4. 1 秒待機する（PTY 出力 → 描画の伝搬を待つ）
5. send-keys で Cmd+T を送信する（2 つ目のセッションを追加）
6. 1 秒待機する（セッション生成 → サイドバー出現 → フォーカス移動を待つ）
7. send-keys で `echo SESSION_TWO` を入力し、Return キーを送信する
8. 1 秒待機する（PTY 出力 → 描画の伝搬を待つ）
9. send-keys で Ctrl+Tab を送信する（1 つ目のセッションへ切り替え）
10. 1 秒待機する（セッション切り替え → 再描画の伝搬を待つ）
11. capture-window でスクリーンショットを撮る（`tmp/006-session-one.png`）
12. send-keys で Ctrl+Tab を送信する（2 つ目のセッションへ戻る）
13. 1 秒待機する
14. capture-window でスクリーンショットを撮る（`tmp/006-session-two.png`）

## 期待結果
- `tmp/006-session-one.png`：画面に "SESSION_ONE" が描画されており、"SESSION_TWO" は表示されていない
- `tmp/006-session-two.png`：画面に "SESSION_TWO" が描画されており、"SESSION_ONE" は表示されていない
- いずれのスクリーンショットでもウィンドウが表示されており、ファイルサイズが 10 KiB 以上（空白でない）

## クリーンアップ
- SDIT プロセスを終了する
- `tmp/006-session-one.png`, `tmp/006-session-two.png` を削除する

## 関連
- Phase 4 計画: `docs/plans/phase4-session-sidebar.md`
- `crates/sdit/src/input.rs` の Ctrl+Tab キーバインド処理
- `crates/sdit-core/src/session/` のアクティブセッション切り替えロジック
