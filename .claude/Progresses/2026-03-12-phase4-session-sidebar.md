# 進捗表

## タスク: Phase 4 — 縦タブ（SessionSidebar）

### Phase 4.1 — データ構造変更
- [x] WindowState を `sessions: Vec<SessionId>`, `active_index`, `sidebar` に変更
- [x] SidebarState を `sdit-session/sidebar.rs` に実装（auto_update, toggle, hit_test, width_px）
- [x] `create_window`, `close_window`, `redraw_session`, `handle_resize` を書き換え
- [x] cargo test 全通過

### Phase 4.2 — キーボードでセッション追加・切替・削除
- [x] `add_session_to_window()` 実装（Cmd+T / Ctrl+Shift+T）
- [x] `remove_active_session()` 実装（Cmd+W / Ctrl+Shift+W）
- [x] `switch_session()` 実装（Ctrl+Tab / Ctrl+Shift+Tab / Cmd+Shift+]/[）
- [x] 非アクティブセッションの PTY 出力は描画スキップ
- [x] リサイズ時に全セッションの Terminal + PTY をリサイズ

### Phase 4.3 — サイドバー描画
- [x] Uniforms に `origin_x` 追加（`_padding` → `origin_x`）
- [x] WGSL シェーダ修正（screen += origin_x）
- [x] サイドバー用 CellPipeline + 2パス描画
- [x] `build_sidebar_cells()` で Catppuccin Mocha カラーのサイドバー生成
- [x] セッション2つ以上で自動出現、1つで消滅

### Phase 4.4 — サイドバー操作
- [x] Cmd+\ / Ctrl+\ でサイドバートグル
- [x] マウスクリックでセッション切替

### Phase 4.5 — セッション切出し
- [x] Cmd+Shift+N でアクティブセッションを新ウィンドウに切出し
- [x] PTY は維持（Surface 差し替えのみ）
- [x] 新ウィンドウで Terminal + PTY をリサイズ
- [x] ロールバック処理（ウィンドウ/GPU作成失敗時）

### Phase 4.6 — ドラッグ＆ドロップ
- [x] サイドバー内ドラッグでタブ順序変更
- [x] マウスクリックでセッション切替
- [ ] ウィンドウ外ドラッグで切出し（Phase 4.5 のキーボード版で代替）
- [ ] Cmd+Shift+M でウィンドウ合体（winit 0.30 の制約で後回し）

### セキュリティレビュー
- [x] M-1: detach ロールバック時のセッション順序不整合 → insert(original_index) で修正
- [x] M-2: ChildExit での削除順序非一貫性 → 統一
- [x] M-3: active_session_id() のパニックリスク → debug_assert 追加
- L-1〜L-4, Info-1〜Info-2: Plan に記録済み

### 完了処理
- [x] Plan ファイル更新
- [x] TODO.md 更新
- [x] Feedback.md 更新
- [x] Progress アーカイブ
- [x] コミット
