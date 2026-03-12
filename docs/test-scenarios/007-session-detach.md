# 007: セッション切り出し

## 目的
Cmd+Shift+N でアクティブセッションを新しいウィンドウに切り出せることを確認する。切り出し後は 2 つの独立したウィンドウが存在し、元ウィンドウのサイドバーが消滅（1 セッション状態に戻る）することを確認する。

## 前提条件
- `cargo build --package sdit`
- `tools/test-utils/build.sh`
- Screen Recording 権限 + OS 再起動
- Accessibility 権限

## 手順
1. SDIT をバックグラウンドで起動する
2. window-info でウィンドウの存在を確認する（最大 15 秒ポーリング）
3. send-keys で Cmd+T を送信する（2 つ目のセッションを追加）
4. 1 秒待機する（セッション生成 → サイドバー出現を待つ）
5. send-keys で `echo DETACHED_SESSION` を入力し、Return キーを送信する
6. 1 秒待機する（PTY 出力 → 描画の伝搬を待つ）
7. send-keys で Cmd+Shift+N を送信する（アクティブセッションを新ウィンドウに切り出す）
8. 2 秒待機する（ウィンドウ生成 → 描画の伝搬を待つ）
9. window-info でウィンドウ一覧を取得し、2 ウィンドウが存在することを確認する
10. 切り出された新しいウィンドウを対象に capture-window でスクリーンショットを撮る（`tmp/007-detached.png`）
11. 元のウィンドウを対象に capture-window でスクリーンショットを撮る（`tmp/007-original.png`）

## 期待結果
- window-info の結果に 2 つのウィンドウエントリが存在する
- `tmp/007-detached.png`：切り出されたウィンドウに "DETACHED_SESSION" が表示されている。サイドバーなし（1 セッション状態）
- `tmp/007-original.png`：元のウィンドウに残ったセッションの内容が表示されている。サイドバーなし（1 セッション状態に戻っている）
- 両ウィンドウとも独立して動作しており、スクリーンショットのファイルサイズが 10 KiB 以上（空白でない）

## クリーンアップ
- SDIT プロセスをすべて終了する（切り出された分も含む）
- `tmp/007-detached.png`, `tmp/007-original.png` を削除する

## 関連
- Phase 4 計画: `docs/plans/phase4-session-sidebar.md`
- `crates/sdit/src/window.rs` のウィンドウ生成ロジック
- `crates/sdit/src/input.rs` の Cmd+Shift+N キーバインド処理
