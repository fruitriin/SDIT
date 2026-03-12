# シナリオ 019: ウィンドウサイズ・位置の永続化

## 概要

ウィンドウのサイズと位置を `session.toml` に保存し、次回起動時に復元できることを確認する。

## 前提条件

- SDIT がビルド済みである
- `~/.local/state/sdit/session.toml` が読み書き可能である

---

## サブシナリオ 019-a: ウィンドウを閉じた後に session.toml へジオメトリが保存される

### 手順

1. SDIT を起動する（初回起動 or session.toml 削除後）
2. ウィンドウを任意のサイズ・位置に変更する
3. ウィンドウを閉じる（Cmd+W または × ボタン）
4. `cat ~/.local/state/sdit/session.toml` を実行する

### 期待結果

- `session.toml` に `[[windows]]` エントリが存在する
- `width`, `height`, `x`, `y` フィールドが記録されている
- 値がウィンドウを閉じる直前のサイズ・位置と一致している

### 自動テスト対象

- `session::persistence::tests::window_geometry_roundtrip` — WindowGeometry の save/load ラウンドトリップ
- `window_ops.rs::close_window` — クローズ時に `AppSnapshot::save` を呼ぶ実装

---

## サブシナリオ 019-b: 起動時に保存ジオメトリからウィンドウが復元される

### 手順

1. サブシナリオ 019-a の手順を完了し、session.toml にジオメトリが保存されていることを確認する
2. SDIT を再起動する
3. 新しく開いたウィンドウのサイズと位置を確認する

### 期待結果

- ウィンドウが前回保存したサイズ・位置で開く
- 端末グリッドが新しいサイズに合わせて計算されている

### 自動テスト対象

- `event_loop.rs::resumed` — `AppSnapshot::load` で geometry を取得し `create_window` に渡す実装

---

## サブシナリオ 019-c: 古い形式の session.toml（windows フィールドなし）でも起動できる

### 手順

1. 以下の内容の `~/.local/state/sdit/session.toml` を作成する:

```toml
[[sessions]]
cwd = "/home/user"
```

2. SDIT を起動する

### 期待結果

- エラーなく起動する
- デフォルトサイズ（800×600）のウィンドウが開く
- session.toml の読み込みに失敗しない（graceful fallback）

### 自動テスト対象

- `session::persistence::tests::backward_compat_no_windows_field` — `#[serde(default)]` によるフィールド省略時のデフォルト動作

---

## サブシナリオ 019-d: 複数ウィンドウのジオメトリが全て保存される

### 手順

1. SDIT を起動する（1つ目のウィンドウ）
2. Cmd+N で2つ目のウィンドウを開く
3. 各ウィンドウを異なるサイズ・位置に変更する
4. 1つ目のウィンドウを閉じる
5. `cat ~/.local/state/sdit/session.toml` を確認する

### 期待結果

- 1つ目のウィンドウを閉じた時点で、残存ウィンドウ（2つ目）のジオメトリが保存される
- `[[windows]]` エントリが1つ存在し、2つ目のウィンドウの値が記録されている

### 備考

- `close_window` は削除後の残存ウィンドウを `collect_window_geometries` で収集して保存する
- 閉じたウィンドウ自体のジオメトリは保存対象外（削除後に収集するため）

---

## サブシナリオ 019-e: ウィンドウリサイズ後のクローズで新サイズが保存される

### 手順

1. SDIT を起動する（デフォルトサイズ 800×600）
2. ウィンドウを大きくリサイズする（例: 1200×800）
3. ウィンドウを閉じる
4. `cat ~/.local/state/sdit/session.toml` を確認する

### 期待結果

- `session.toml` にリサイズ後のサイズ（1200×800 相当の論理サイズ）が保存されている
- `x`, `y` も現在の位置が保存されている

---

## 関連ユニットテスト

| テスト名 | 場所 | 検証内容 |
|---|---|---|
| `window_geometry_roundtrip` | `persistence.rs` | WindowGeometry の save/load ラウンドトリップ |
| `backward_compat_no_windows_field` | `persistence.rs` | windows フィールドなし TOML の後方互換性 |
| `empty_windows_list_roundtrip` | `persistence.rs` | windows が空のケースのラウンドトリップ |
| `roundtrip_save_load` | `persistence.rs` | sessions を含む AppSnapshot のラウンドトリップ |
| `load_nonexistent_returns_default` | `persistence.rs` | ファイル不在時のデフォルト返却 |
| `load_corrupted_returns_default` | `persistence.rs` | 破損 TOML 時のデフォルト返却 |
| `default_path_exists` | `persistence.rs` | デフォルトパスに "sdit/session.toml" が含まれる |
| `default_snapshot_is_empty` | `persistence.rs` | デフォルト AppSnapshot が空 |

## ユニットテスト実行結果（2026-03-13）

```
running 8 tests
test session::persistence::tests::default_snapshot_is_empty ... ok
test session::persistence::tests::load_nonexistent_returns_default ... ok
test session::persistence::tests::default_path_exists ... ok
test session::persistence::tests::load_corrupted_returns_default ... ok
test session::persistence::tests::backward_compat_no_windows_field ... ok
test session::persistence::tests::empty_windows_list_roundtrip ... ok
test session::persistence::tests::roundtrip_save_load ... ok
test session::persistence::tests::window_geometry_roundtrip ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 161 filtered out; finished in 0.00s
```

## 注記

- GUI テスト（019-a〜e）はウィンドウサーバー環境が必要なため、CI での自動化は別途検討が必要
- サブシナリオ 019-d の挙動（閉じたウィンドウ自体は保存対象外）は仕様通り。次回起動時は残存ウィンドウのジオメトリから復元される
