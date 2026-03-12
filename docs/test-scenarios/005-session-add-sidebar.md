# 005: セッション追加とサイドバー出現

## 目的
Cmd+T でセッションを追加すると縦タブサイドバーが自動出現し、セッションを 1 つに戻すとサイドバーが消滅することを確認する。

## 前提条件
- `cargo build --package sdit`
- `tools/test-utils/build.sh`
- Screen Recording 権限 + OS 再起動
- Accessibility 権限

## 手順
1. SDIT をバックグラウンドで起動する
2. window-info でウィンドウの存在を確認する（最大 15 秒ポーリング）
3. capture-window でスクリーンショットを撮る（`tmp/005-sidebar-before.png`）：サイドバーが存在しない初期状態
4. send-keys で Cmd+T を送信する（2 つ目のセッションを追加）
5. 1 秒待機する（セッション生成 → サイドバー出現 → 再描画の伝搬を待つ）
6. capture-window でスクリーンショットを撮る（`tmp/005-sidebar-shown.png`）：サイドバー出現状態
7. send-keys で Cmd+W を送信する（アクティブセッションを閉じて 1 セッションに戻す）
8. 1 秒待機する（セッション破棄 → サイドバー消滅 → 再描画の伝搬を待つ）
9. capture-window でスクリーンショットを撮る（`tmp/005-sidebar-after.png`）：サイドバーが消えた状態

## 期待結果
- `tmp/005-sidebar-before.png`：ウィンドウが表示されている。サイドバー領域が描画されておらず、ウィンドウ幅全体が端末グリッドに使われている
- `tmp/005-sidebar-shown.png`：サイドバー領域が描画されている（ウィンドウ左端に縦タブバーが出現する、または端末グリッド幅が狭まる）
- `tmp/005-sidebar-after.png`：サイドバーが消滅し、`tmp/005-sidebar-before.png` と同等の状態に戻っている

## クリーンアップ
- SDIT プロセスを終了する
- `tmp/005-sidebar-before.png`, `tmp/005-sidebar-shown.png`, `tmp/005-sidebar-after.png` を削除する

## 関連
- Phase 4 計画: `docs/plans/phase4-session-sidebar.md`
- `crates/sdit-core/src/session/` の `SessionSidebar` 状態管理
- `crates/sdit/src/window.rs` のサイドバー表示ロジック
