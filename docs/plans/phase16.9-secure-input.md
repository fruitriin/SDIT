# Phase 16.9: Secure Keyboard Entry（macOS）

**概要**: macOS の Secure Input API を使ってパスワード入力中に他のアプリがキーログできなくする機能。

**状態**: **完了**

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| Config に secure_input 設定追加 | `auto: bool`（デフォルト false）、パスワードプロンプト検出時に自動有効化 | sdit-core (`config/mod.rs`) | **完了** |
| macOS API 呼び出し | `EnableSecureEventInput()` / `DisableSecureEventInput()` | sdit (`secure_input.rs`) | **完了** |
| Action::ToggleSecureInput 追加 | キーバインドでトグル可能 | sdit-core (`config/keybinds.rs`), sdit (`event_loop.rs`) | **完了** |
| テスト | 設定デシリアライズ | sdit-core | **完了** |

## 設定例

```toml
[security]
auto_secure_input = false
```

## 参照

- `refs/ghostty/src/config/Config.zig` — macos-auto-secure-input
