# Phase 10.1: 設定Hot Reload

**状態**: **完了** (2026-03-13)

**概要**: 設定ファイル変更時に自動的にリロードする機能を実装する。

## タスク

| タスク | 詳細 | 変更先クレート | 状態 |
|---|---|---|---|
| ファイル監視 | `notify` クレートで `sdit.toml` の変更を監視 | sdit | 完了 |
| 差分適用 | フォント・カラー・キーバインドの変更をリロード | sdit | 完了 |

## 実装内容

- `config_watcher.rs`: `notify` v7 の `RecommendedWatcher` で親ディレクトリを監視（NonRecursive）
- デバウンス: 300ms、`Arc<Mutex<Option<Instant>>>` + `Arc<AtomicBool>` タイマーガード
- `SditEvent::ConfigReloaded` → `apply_config_reload()` で差分適用（フォント/カラー/キーバインド）
- フォント変更時: `FontContext` 再構築 + Atlas クリア + 全セッション resize
- カラー変更時: `ResolvedColors` 再生成
- キーバインド変更時: `self.config` 置換

## 依存関係

Phase 9.2（キーバインドカスタマイズ。メニューとショートカットの一元管理が前提）

## 新規依存クレート

`notify = "7"`

## セキュリティレビュー結果

### 修正済み

- **M-1 (Medium)**: デバウンスタイマースレッド重複起動の競合条件 → `AtomicBool` フラグで排他制御に修正

### 記録（Low/Info — 将来対応）

- **L-1 (Low)**: `PoisonError::into_inner` で Mutex ポイズニングを無視 — 現状シングルスレッド書き込みのため実質安全
- **L-2 (Low)**: `unwrap()` on `watcher.watch()` 失敗がプロセス継続に影響 — 実際には `Option` で `None` 返却済みなので問題なし
- **L-3 (Low)**: 設定リロード時のバリデーションエラーがサイレント — `Config::load()` 内で warn ログ出力済み
- **I-1 (Info)**: `DEBOUNCE_DURATION` のハードコード — 将来的に設定可能にする余地あり
