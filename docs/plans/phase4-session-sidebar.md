# Phase 4 — 縦タブ（SessionSidebar）

## 目的

セッションが2つ以上になったときに縦タブバー（SessionSidebar）を表示し、
Chrome-like なタブ合体・切出しUXを実現する。

## 前提条件

- Phase 3 のSDI複数ウィンドウが完了していること

## 設計概要

### 1 Window : N Sessions への移行

```
【現在】
WindowState { session_id: SessionId }                // 1:1
session_to_window: HashMap<SessionId, WindowId>      // 1:1

【Phase 4 後】
WindowState { sessions: Vec<SessionId>, active_index: usize, sidebar_visible: bool }
session_to_window: HashMap<SessionId, WindowId>      // N:1
```

不変条件:
- `sessions.len() >= 1`（空にならない）
- `sessions.len() == 1` → `sidebar_visible = false`（SDI状態）
- `sessions.len() >= 2` → `sidebar_visible = true`（自動出現、手動トグル可能）

### サイドバー描画

既存の CellPipeline（インスタンス描画）を流用。サイドバーは CellVertex 列として描画。

- サイドバー幅: 固定 `SIDEBAR_WIDTH_CELLS = 20` セル幅
- Uniforms に `origin_x` を追加してターミナル描画領域をオフセット
- サイドバーとターミナルを同一レンダーパスで2回 draw

### レイアウト

```
┌──────────────────────────────────────────┐
│ [Sidebar]  [Terminal Area]               │
│  Session1  $ ls -la                      │
│ >Session2  $ echo hello                  │
│            hello                         │
│            $                             │
└──────────────────────────────────────────┘
```

---

## サブフェーズ

### Phase 4.1 — データ構造変更（1 Window : N Sessions）

動作は Phase 3 と同一だが、内部構造が Vec ベースになる。

**変更ファイル:** `main.rs`, `sidebar.rs`

- WindowState を `sessions: Vec<SessionId>`, `active_index: usize`, `sidebar_visible: bool` に変更
- SidebarState を `sdit-session/sidebar.rs` に定義
- `create_window`, `close_window`, `redraw_session`, `handle_resize` を書き換え
- [x] 完了

### Phase 4.2 — キーボードでセッション追加・切替・削除

**変更ファイル:** `main.rs`, `sidebar.rs`

| キー | 動作 |
|---|---|
| Cmd+T / Ctrl+Shift+T | 同一ウィンドウに新規セッション追加 |
| Cmd+W / Ctrl+Shift+W | アクティブセッションを閉じる |
| Ctrl+Tab / Cmd+Shift+] | 次のセッションに切替 |
| Ctrl+Shift+Tab / Cmd+Shift+[ | 前のセッションに切替 |

- `add_session_to_window()`, `remove_active_session()`, `switch_session()` 実装
- リサイズ時は全セッションの Terminal + PTY をリサイズ
- PTY 出力は非アクティブセッションでは描画スキップ
- [x] 完了

### Phase 4.3 — サイドバー描画

**変更ファイル:** `pipeline.rs`, `cell.wgsl`, `main.rs`, `sidebar.rs`

- Uniforms に `origin_x` 追加（`_padding` → `origin_x`）
- WGSL: `screen += vec2(origin_x, 0.0)`
- サイドバー用 CellPipeline を WindowState に追加
- 2パス描画: サイドバー（origin_x=0）+ ターミナル（origin_x=sidebar_width_px）
- セッション2つ以上で自動出現、1つで消滅
- [x] 完了

### Phase 4.4 — サイドバー操作（クリック・トグル）

**変更ファイル:** `main.rs`, `sidebar.rs`

- Cmd+\ でサイドバートグル（単一セッションでも表示可能）
- マウスクリックでセッション切替（x < sidebar_width_px → ヒットテスト）
- [x] 完了

### Phase 4.5 — セッション切出し（キーボード版）

**変更ファイル:** `main.rs`, `sidebar.rs`

- Cmd+Shift+N: アクティブセッションを新しいウィンドウに切り出す
- PTY は切れない（Surface 差し替えのみ）
- 新ウィンドウで Terminal + PTY をリサイズ
- [x] 完了

### Phase 4.6 — ドラッグ＆ドロップ

**変更ファイル:** `main.rs`, `sidebar.rs`

- サイドバー内ドラッグでタブ順序変更
- ウィンドウ外ドラッグで切出し → Phase 4.5 のキーボード版（Cmd+Shift+N）で代替
- Cmd+Shift+M でウィンドウ合体（キーボード代替）→ 未実装（winit 0.30 の制約で後回し）
- [x] 完了（サイドバー内ドラッグ + クリック切替を実装）

## 対象クレート

- `crates/sdit-session/` (`sidebar.rs`)
- `crates/sdit-render/` (`pipeline.rs`, `cell.wgsl`)
- `crates/sdit/` (`main.rs`)

## セキュリティ考慮事項

- セッション移動時の `session_to_window` 整合性（remove → insert をアトミックに）
- セッション名の長さ制限（`SIDEBAR_WIDTH_CELLS` でトランケート）
- マウス座標の負値・巨大値への防御

### セキュリティレビュー結果

| ID | 重要度 | 概要 | 対応 |
|---|---|---|---|
| M-1 | Medium | detach ロールバック時のセッション順序不整合 | **修正済み**: `insert(original_index)` で元位置に復元 |
| M-2 | Medium | ChildExit での削除順序の非一貫性 | **修正済み**: ws.sessions → session_to_window → session_mgr の順に統一 |
| M-3 | Medium | `active_session_id()` のパニックリスク | **修正済み**: `debug_assert!` 追加 + 呼び出しサイトで空チェック |
| L-1 | Low | マウス座標 f64→f32 変換 | 実害なし（許容範囲） |
| L-2 | Low | セッション名のラベルクリップ | 将来セッション名カスタマイズ時に対応 |
| L-3 | Low | active_index 越境時のサイレント描画不具合 | M-3 の debug_assert で検出可能 |
| L-4 | Low | スレッド spawn の unwrap | 実質リスクなし |
| Info-1 | Info | デタッチ直後の一瞬黒フレーム | 機能上問題なし |
| Info-2 | Info | update_cells() での容量チェック欠如 | 呼び出し側で ensure_capacity 済み |

## 参照

- `refs/zellij/default-plugins/tab-bar/src/`
- `refs/wezterm/wezterm-gui/src/glwindow.rs`
- `refs/ghostty/src/Surface.zig`
- CLAUDE.md「縦タブへの適用」セクション

## 完了条件（Phase 4 全体）

- [x] 1 Window : N Sessions が動作する
- [x] セッション2つ以上で縦タブバーが自動出現
- [x] セッション1つに戻るとタブバー消滅（SDI状態復帰）
- [x] Cmd+\ でサイドバートグル
- [x] サイドバークリックでセッション切替
- [x] Cmd+T で同一ウィンドウにセッション追加
- [x] Cmd+W でアクティブセッション削除
- [x] Ctrl+Tab / Ctrl+Shift+Tab でセッション切替
- [x] Cmd+Shift+N でセッション切出し（PTY は切れない）
- [x] ドラッグでタブ順序変更（サイドバー内）
- [ ] ウィンドウ外ドラッグで切出し → Phase 4.5 のキーボード版で代替
- [x] `cargo test` + `scripts/check.sh` 全通過
- [x] セキュリティレビュー完了（Medium 3件修正済み）
