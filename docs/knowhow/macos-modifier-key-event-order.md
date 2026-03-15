# macOS モディファイアキーのイベント順序

## 発見日
2026-03-15

## 問題
Cmd+C でコピーできない。ダブルクリックで selection を設定した後、Cmd キーを押した瞬間に selection が消える。

## 根本原因
winit の macOS 実装では、Cmd キーを押すと以下の順序でイベントが来る:

1. `WindowEvent::KeyboardInput { key: Named(Super), state: Pressed }` — **modifiers はまだ更新前**
2. `WindowEvent::ModifiersChanged` — ここで `modifiers.super_key() = true` に更新

つまり `KeyboardInput` ハンドラ内で `self.modifiers.super_key()` を参照しても、Cmd キー自体の Press イベントでは **false** が返る。

## 影響
「Cmd/Ctrl 修飾付きの入力では selection をクリアしない」というガードが、Cmd キー自体の Press では機能しない。

## 解決策
モディファイアキー自体（`Named(Super | Control | Shift | Alt)`）の `KeyboardInput` では selection をクリアしない:

```rust
let is_modifier_key = matches!(
    key_event.logical_key,
    Key::Named(NamedKey::Super | NamedKey::Control | NamedKey::Shift | NamedKey::Alt)
);
if !is_modifier_key && !self.modifiers.super_key() && !self.modifiers.control_key() {
    self.selection = None;
}
```

## 関連
- macOS メニューアクセラレータ: Cmd+C は winit の `KeyboardInput` では来ない場合がある（メニューが消費する）。代わりに `MenuEvent` 経由で `Action::Copy` が来る
- Alacritty/Ghostty もモディファイアキー単独の入力を特別扱いしている

## デバッグ手法
`self.selection = None` の全箇所に条件付き `log::warn!` を追加して、どこで消されているか特定する。タイトルバーに selection 状態を表示するのも有効。
