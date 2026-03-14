# Phase 16.9: Secure Keyboard Entry（macOS）

**概要**: macOS の Secure Input API を使ってパスワード入力中に他のアプリがキーログできなくする機能。

**状態**: 未着手

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| Config に secure_input 設定追加 | `auto: bool`（デフォルト false）、パスワードプロンプト検出時に自動有効化 | sdit-core (`config/mod.rs`) | 未着手 |
| macOS API 呼び出し | `EnableSecureEventInput()` / `DisableSecureEventInput()` | sdit (`event_loop.rs`) | 未着手 |
| Action::ToggleSecureInput 追加 | キーバインドでトグル可能 | sdit-core (`config/keybinds.rs`), sdit (`event_loop.rs`) | 未着手 |
| テスト | 設定デシリアライズ | sdit-core | 未着手 |

## 設定例

```toml
[security]
auto_secure_input = false
```

## 参照

- `refs/ghostty/src/config/Config.zig` — macos-auto-secure-input
