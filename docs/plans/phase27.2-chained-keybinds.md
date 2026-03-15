# Phase 27.2: Chained Keybinds（複数アクション連鎖）

## 要望

1 つのキーバインドで複数のアクションを順番に実行できるようにする。

現在: キーバインド 1 つに対して 1 アクションしか割り当てられない。
Chained keybinds を使うと、キー 1 つで複数のアクションを順番に実行できる。

Ghostty: `chain=<action>` 構文（Binding Parser）

## ユースケース

```toml
[[keybinds]]
key = "f"
mods = "ctrl"
action = "Search"
action_chain = ["ScrollToBottom"]
# → Ctrl+F で検索バーを開きつつ、表示を最下部にスクロールする
```

```toml
[[keybinds]]
key = "k"
mods = "super"
action = "ScrollToBottom"
action_chain = ["Copy"]
# → Cmd+K でスクロールしてからコピー
```

## 実装方針

1. `KeyBinding` に `action_chain: Vec<Action>` フィールドを追加
2. `resolve_action()` の戻り値を `Option<(Action, Vec<Action>, bool)>` に変更（Action = primary, Vec<Action> = chain, bool = unconsumed）
3. `event_loop.rs`: primary action と chain actions を順番に実行する

シンプルな実装として、`Action` ではなく `Vec<Action>` 全体で表現:

```rust
// KeyBinding
pub action: Action,           // 主アクション（既存）
pub action_chain: Vec<Action>, // 追加アクション（新規、デフォルト: 空）
```

## 変更対象

- `crates/sdit-core/src/config/keybinds.rs` — `KeyBinding.action_chain: Vec<Action>` 追加
- `crates/sdit/src/input.rs` — `resolve_action()` でチェーンを返す
- `crates/sdit/src/event_loop.rs` — チェーンアクションを順番に実行

## セキュリティ影響

なし（アクション数の上限は 16 等に制限して DoS を防ぐ）

## 参照

- Ghostty: `refs/ghostty/src/input/Binding.zig` `chain` 構文
- Phase 26.1: unconsumed: の実装パターン
