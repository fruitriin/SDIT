# Phase 17.6: セッション復帰

**概要**: アプリケーション終了時にウィンドウ・タブのレイアウト情報を保存し、次回起動時に自動復帰する。

**状態**: **完了**

## セキュリティレビュー結果（Phase 17.5〜17.6 共通）

| 重要度 | ID | 概要 | 対応 |
|---|---|---|---|
| High | H-1 | session.toml ファイルサイズ制限なし（DoS） | **修正済み** — MAX_SESSION_FILE_SIZE (1MB) を追加 |
| High | H-2 | CWD バリデーション未実施（制御文字、相対パス） | **修正済み** — SessionRestoreInfo::validated() を追加 |
| Medium | M-1 | active_session_index バウンダリチェックなし | **修正済み** — validated_active_index() を追加 |
| Medium | M-2 | コマンドパレット入力の制御文字混入 | **修正済み** — push_str() で制御文字フィルタ追加 |
| Medium | M-3 | カスタム名のサニタイズ未実施 | **修正済み** — validated() で制御文字・長さチェック |
| Medium | M-4 | コマンドパレットの ASCII lowercase のみ対応 | UX 問題として許容（国際化は将来対応） |
| Low | L-1 | ウィンドウ座標のマルチモニタ検証 | 既存の validated() で対応 |
| Info | I-1 | コマンドパレット状態の復元不要 | 仕様として確認済み |

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
