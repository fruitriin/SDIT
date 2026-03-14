# Phase 19.1: 環境変数注入 + 初期ワーキングディレクトリ

**概要**: ターミナルセッション起動時に環境変数を注入する機能と、カスタムの初期ワーキングディレクトリを設定する機能を追加する。

**状態**: 未着手

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| Config に env 設定追加 | HashMap<String, String> で環境変数 | sdit-core (`config/mod.rs`) | 未着手 |
| Config に working_directory 設定追加 | Option<String> でカスタム初期ディレクトリ | sdit-core (`config/mod.rs`) | 未着手 |
| PTY 起動時に環境変数を注入 | Command::env() で設定 | sdit-core (`pty/mod.rs`) | 未着手 |
| PTY 起動時にワーキングディレクトリを設定 | Command::current_dir() で設定 | sdit-core (`pty/mod.rs`) | 未着手 |
| テスト | 設定デシリアライズ + バリデーション | sdit-core | 未着手 |

## 設定例

```toml
[env]
TERM_PROGRAM = "sdit"
COLORTERM = "truecolor"

[window]
working_directory = "~/Projects"  # inherit_working_directory より優先
```

## セキュリティ考慮

- 環境変数のキー/値に制御文字が含まれていないかバリデーション
- PATH の上書きは警告を出す（ログレベル warn）
- エントリ数の上限（64）

## 参照

- `refs/ghostty/src/config/Config.zig` — env, working-directory
