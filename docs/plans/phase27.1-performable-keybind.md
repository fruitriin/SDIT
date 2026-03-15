# Phase 27.1: `performable:` キーバインドプレフィックス

## 要望

キーバインドのアクションが「実行可能」なときのみキーを消費し、
実行不可な場合は通常の入力として PTY に転送する。

現在: アクションが実行できなくても（例: 選択テキストがないのに Copy）キーが消費されてしまう。
`performable:` プレフィックスを付けると、アクションが実行可能な場合のみ消費する。

Ghostty: `Binding.Flags.performable` フラグ（Binding.zig）

## ユースケース

- `Cmd+C` を `performable:Copy` にすることで、選択テキストがある場合のみコピー動作、
  なければ通常の Ctrl+C として PTY に送る（vim の <C-c> で現在のコマンドをキャンセル等）
- `Cmd+V` を `performable:Paste` にして、クリップボードが空なら PTY に v を送る

## 実装方針

1. `KeyBinding` に `performable: bool` フィールドを追加（`unconsumed` と類似）
2. アクション実行前に「実行可能か」を判定する関数 `Action::is_performable(app) -> bool` を追加
3. `performable = true` かつ実行不可の場合は、PTY にキーを転送してアクションをスキップ

### 実行可能判定の例

| Action | 実行可能条件 |
|---|---|
| `Copy` | 選択テキストが存在する |
| `Paste` | クリップボードに内容がある |
| `SearchNext` / `SearchPrev` | 検索バーが開いている |
| その他 | 常に実行可能（true を返す） |

## 変更対象

- `crates/sdit-core/src/config/keybinds.rs` — `KeyBinding.performable: bool` 追加
- `crates/sdit/src/input.rs` — `resolve_action()` 戻り値に performable フラグ追加
- `crates/sdit/src/event_loop.rs` — performable かつ実行不可のとき PTY 転送
- `crates/sdit/src/action_handlers.rs` — `can_perform(action, app, window_id) -> bool` を追加

## 実装結果（2026-03-15 完了）

- `KeyBinding.performable: bool` を追加（デフォルト: false）
- `resolve_action()` → `Option<(Action, bool, bool)>` (unconsumed, performable) に変更
- `can_perform(action, window_id) -> bool` を action_handlers.rs に追加
- `event_loop.rs`: performable かつ実行不可の場合は PTY 転送にフォールスルー
- テスト 448 件 PASS（performable テスト 3 件追加）

## セキュリティ影響

なし（セキュリティレビュー済み、Critical/High 0件）

## 参照

- Ghostty: `refs/ghostty/src/input/Binding.zig` `performable` フラグ
- Phase 26.1: unconsumed: の実装パターン
