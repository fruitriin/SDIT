# Phase 19.1: 環境変数注入 + 初期ワーキングディレクトリ

**概要**: ターミナルセッション起動時に環境変数を注入する機能と、カスタムの初期ワーキングディレクトリを設定する機能を追加する。

**状態**: 完了

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| Config に env 設定追加 | HashMap<String, String> で環境変数 | sdit-core (`config/mod.rs`) | 完了 |
| Config に working_directory 設定追加 | Option<String> でカスタム初期ディレクトリ | sdit-core (`config/mod.rs`) | 完了 |
| PTY 起動時に環境変数を注入 | spawn_session_with_cwd で env を適用 | sdit (`app.rs`) | 完了 |
| PTY 起動時にワーキングディレクトリを設定 | 引数 → config.window.working_directory の順で解決 | sdit (`app.rs`) | 完了 |
| テスト | 設定デシリアライズ + バリデーション | sdit-core | 完了 |

## 設定例

```toml
[env]
TERM_PROGRAM = "sdit"
COLORTERM = "truecolor"

[window]
working_directory = "~/Projects"  # inherit_working_directory より優先
```

## セキュリティ考慮

- 環境変数のキー/値に制御文字が含まれていないかバリデーション（含む場合は warn してスキップ）
- PATH の上書きは警告を出す（ログレベル warn）
- エントリ数の上限（64）

## 実装メモ

- `spawn_session_with_cwd` で `config.env` を最大 64 エントリ反映（PTY デフォルト環境変数に追加）
- `working_directory` は `~` 展開に対応
- 解決順: 引数 `working_dir` → `config.window.working_directory`

## 参照

- `refs/ghostty/src/config/Config.zig` — env, working-directory
