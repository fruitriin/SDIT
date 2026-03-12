# テストシナリオ 022: Config Serialize + TOML テンプレート生成

## 概要

Phase 11.3 で実装された Config Serialize derive、`Config::save()`、`Config::save_with_comments()`、
および `Action::Preferences` ハンドラ（設定ファイル未存在時のテンプレート自動生成）を確認する。

## 前提条件

- `cargo test` が通過していること

## テスト手順

### 022-1: Config Serialize — デフォルト設定の保存と再ロード

**ユニットテスト対応**: `config::tests::config_save_and_load_roundtrip`

1. `Config::default()` を生成する
2. `Config::save(&path)` で TOML ファイルに書き出す
3. `Config::load(&path)` でファイルを再読み込みする
4. **期待**: フォントサイズ・フォントファミリー等の値が一致する

### 022-2: Config Serialize — コメント付きテンプレートの生成と再パース

**ユニットテスト対応**: `config::tests::config_save_with_comments_is_parseable`

1. `Config::default()` を生成する
2. `Config::save_with_comments(&path)` でコメント付き TOML ファイルを書き出す
3. 生成されたファイルの内容を確認する
4. **期待**: `# SDIT` ヘッダーコメントが含まれる
5. **期待**: `[font]`、`[colors]`、`[[keybinds]]` の各セクションにコメントが挿入される
6. **期待**: `Config::load(&path)` でパースに成功し、デフォルト値と一致する

### 022-3: Config Serialize — 全構造体のシリアライズ可能性

1. `Config::default()` を生成する
2. `toml::to_string_pretty(&config)` でシリアライズする
3. **期待**: エラーなくシリアライズが完了する
4. **期待**: `[font]`, `[colors]`, `[[keybinds]]` セクションが含まれる
5. 結果の TOML 文字列を `toml::from_str::<Config>()` でデシリアライズする
6. **期待**: デシリアライズに成功する（ラウンドトリップ）

### 022-4: Action::Preferences — 設定ファイル未存在時にテンプレート自動生成

**GUI テスト（手動確認）**

1. 設定ファイル（`~/.config/sdit/sdit.toml`）が存在しないことを確認する
2. SDIT を起動する
3. メニューバーの「SDIT > Preferences…」（または Cmd+,）を実行する
4. **期待**: `~/.config/sdit/sdit.toml` が自動生成される
5. **期待**: 生成されたファイルに `# SDIT Terminal Configuration` ヘッダーが含まれる
6. **期待**: エディタ（デフォルトアプリケーション）でファイルが開かれる

### 022-5: Action::Preferences — 設定ファイル既存時はそのまま開く

**GUI テスト（手動確認）**

1. `~/.config/sdit/sdit.toml` が既に存在することを確認する
2. SDIT を起動する
3. Cmd+, を実行する
4. **期待**: 既存の設定ファイルの内容が保持される（上書きされない）
5. **期待**: エディタでファイルが開かれる

### 022-6: Config::save_with_comments — セクションコメントの確認

1. `Config::default()` を生成し `save_with_comments()` を呼び出す
2. 生成されたファイルの内容を行単位で確認する
3. **期待**: `[font]` セクションの直前に以下コメントが挿入される:
   - `# ── Font ───...`
   - `# family: font family name ...`
   - `# size: font size in pixels ...`
   - `# line_height: line height multiplier ...`
   - `# fallback_families: list of fallback font families ...`
4. **期待**: `[colors]` セクションの直前に Color セクションコメントが挿入される
5. **期待**: `[[keybinds]]` セクションの直前に Keybinds セクションコメントが挿入される

## 自動テスト

`cargo test` で以下のユニットテストが実行される:

- `config::tests::config_save_and_load_roundtrip` — save + load のラウンドトリップ確認
- `config::tests::config_save_with_comments_is_parseable` — コメント付きテンプレートのパース可能性確認
- `config::tests::default_config_is_valid` — デフォルト設定の正常性確認
- `config::tests::load_nonexistent_returns_default` — 存在しないファイルのロード時デフォルト返却確認
- `config::tests::deserialize_full_config` — フル設定のデシリアライズ確認
- `config::tests::deserialize_empty_uses_defaults` — 空設定のデフォルト値確認

## 実装メモ

- `Config` 構造体全体と `FontConfig`、`ColorConfig`、`KeybindConfig` に `Serialize` derive を追加
- `Config::save()` は `toml::to_string_pretty()` + `std::fs::write()` で原子的に保存
- `Config::save_with_comments()` は TOML 文字列を行単位でスキャンし、セクションヘッダー前にコメントを挿入
- `Action::Preferences` ハンドラは `path.exists()` チェックで未存在時のみ `save_with_comments()` を呼び出す
- `open::that()` で OS デフォルトのエディタでファイルを開く
