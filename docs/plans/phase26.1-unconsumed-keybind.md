# Phase 26.1: `unconsumed:` キーバインドプレフィックス

## 要望

キーバインドにアクションを割り当てつつ、そのキーイベントをターミナルにも転送できるようにする。

現在: キーバインドで捕捉したキーは SDIT が消費し、ターミナルアプリには届かない。
`unconsumed:` プレフィックスを付けると、アクションを実行しつつキーをターミナルに転送する。

Ghostty: `unconsumed:` プレフィックス（Config.zig:1711）

## ユースケース

- `Ctrl+D` をセッション終了に割り当てつつ、シェルにも EOF を送りたい
- `Cmd+K` でスクロールバッファをクリアしつつ、アプリへもキーを渡したい

## 実装方針

1. キーバインドの action 文字列に `"unconsumed:"` プレフィックスを認識させる
2. `unconsumed:` 付きアクション実行後、元のキーイベントを PTY に転送する
3. config: `bind = { key = "ctrl+d", action = "unconsumed:close_session" }` のような構文

### 実装箇所

- `crates/sdit-core/src/config/keybinds.rs` — Action パース時に `unconsumed:` を検出し、フラグを保持
- `crates/sdit/src/input.rs` — アクション実行後に `should_forward_to_pty` フラグを確認して転送

## 変更対象

- `crates/sdit-core/src/config/keybinds.rs` — `KeyBinding` に `unconsumed: bool` フラグ追加
- `crates/sdit/src/input.rs` — アクション実行後の PTY 転送ロジック
- `docs/manuals/keybinds.md` — `unconsumed:` 使用例を追加

## セキュリティ影響

なし

## 参照

- Ghostty: `refs/ghostty/src/config/Config.zig` L1711 `unconsumed:`
