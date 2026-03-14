# Phase 17.6: セッション復帰

**概要**: アプリケーション終了時にウィンドウ・タブのレイアウト情報を保存し、次回起動時に自動復帰する。

**状態**: 未着手

## 背景

- Phase 10.2 でウィンドウの位置・サイズの永続化は実装済み
- セッション（タブ）構成・作業ディレクトリの復帰はまだ未実装
- PTY の状態（スクロールバックバッファ等）の復帰は対象外（PTY は新規起動）

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| セッションスナップショット拡張 | ウィンドウごとのタブ一覧 + 各タブの CWD を記録 | sdit-core (`session/persistence.rs`) | 未着手 |
| 終了時保存 | アプリケーション終了（Quit）時にスナップショットをファイル保存 | sdit (`event_loop.rs`) | 未着手 |
| 起動時復帰 | 保存されたスナップショットからウィンドウ・タブ・CWD を復元して PTY を起動 | sdit (`app.rs`, `window_ops.rs`) | 未着手 |
| Config に restore_session 設定追加 | `bool`（デフォルト true） | sdit-core (`config/mod.rs`) | 未着手 |
| テスト | スナップショットのシリアライズ/デシリアライズ | sdit-core | 未着手 |

## 設定例

```toml
[window]
restore_session = true
```

## 参照

- `refs/zellij/zellij-server/src/session_layout_metadata.rs` — セッションレイアウトメタデータ
- `refs/wezterm/wezterm-mux/src/` — Mux 層のセッション管理
