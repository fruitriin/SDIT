# Phase 11.3: GUI設定画面 → 設定シリアライズ + TOML テンプレート

**概要**: Config の Serialize 対応と、コメント付き TOML テンプレートの自動生成。
egui による GUI 設定画面は見送り、TOML 直接編集 + Hot Reload で代替する。

**状態: 完了**

## 設計判断

当初は egui-wgpu 統合による GUI 設定画面を計画していたが、以下の理由で方針を変更した:

- Hot Reload（Phase 10.1）が既に動作しており、TOML 編集 → 即時反映のフローが確立済み
- egui-wgpu 統合は wgpu パイプラインへの非自明な変更が必要で、複雑性コストが高い
- SDIT の「シンプルを守る」哲学に合致する

代替として「Option B+」を採用: Config の Serialize 対応 + コメント付き TOML テンプレート生成。

## タスク

| タスク | 詳細 | 変更先クレート | 状態 |
|---|---|---|---|
| Config Serialize 対応 | Config, FontConfig, ColorConfig, ThemeName, KeybindConfig, KeyBinding, Action に Serialize derive | sdit-core (config/) | 完了 |
| Config::save() | TOML ファイルへの書き出し | sdit-core (config/mod.rs) | 完了 |
| Config::save_with_comments() | コメント付きテンプレート生成（ファイル未存在時のみ書き出し） | sdit-core (config/mod.rs) | 完了 |
| Action::Preferences 連携 | Cmd+, で設定ファイルが存在しなければテンプレートを生成し、エディタで開く | sdit (event_loop.rs) | 完了 |
| テスト追加 | roundtrip テスト、コメント付き save テスト | sdit-core (config/mod.rs) | 完了 |

## 依存関係

- Phase 10.1（Hot Reload）
- Phase 9.2（キーバインドカスタマイズ）

## 新規依存クレート

`open` (エディタでファイルを開く — Phase 11.1 で追加済み)

## 変更ファイル

- `crates/sdit-core/src/config/mod.rs`: `Serialize` derive 追加、`save()`, `save_with_comments()` メソッド追加、テスト追加
- `crates/sdit-core/src/config/font.rs`: `Serialize` derive 追加
- `crates/sdit-core/src/config/color.rs`: `Serialize` derive 追加 (`ColorConfig`, `ThemeName`)
- `crates/sdit-core/src/config/keybinds.rs`: `Serialize` derive 追加 (`Action`, `KeyBinding`, `KeybindConfig`)
- `crates/sdit/src/event_loop.rs`: `Action::Preferences` ハンドラで `save_with_comments` を呼び出し

## セキュリティレビュー結果

| 重要度 | ID | 内容 | 対応 |
|---|---|---|---|
| Medium | M-1 | `path.exists()` + `save_with_comments()` の TOCTOU 競合 | **修正済み**: `OpenOptions::create_new(true)` で排他的作成に変更。呼び出し側の `if !path.exists()` ガードを削除 |
| Low | L-1 | `save()` は既存ファイルを無条件上書き | 現在 `save()` の呼び出し箇所はないため実害なし。将来使用時にバックアップを検討 |
| Info | I-1 | `PoisonError::into_inner` の使用 | HashMap 操作は軽量で整合性リスクは低い（Phase 11.2 と同じ判断） |
| Info | I-2 | `open::that()` の外部コマンド実行 | ユーザー操作起点（Cmd+,）でのみ発動。リスク許容範囲 |
