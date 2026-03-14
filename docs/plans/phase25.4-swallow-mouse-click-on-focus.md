# Phase 25.4: ウィンドウフォーカス取得時のマウスクリック抑制

## 要望

SDIT のウィンドウがフォーカスされていない状態でクリックしたとき、
そのクリックをフォーカス取得のみに使い、ターミナルへの入力として扱わないようにする。

WezTerm: `swallow_mouse_click_on_window_focus = false`（デフォルト: false で現行と同様）
Ghostty: macOS ではデフォルトでフォーカスクリックを swallow しない

## 現状

現在の SDIT は、フォーカスされていないウィンドウをクリックすると
フォーカス取得 + クリック入力の両方が発生する（標準 macOS 挙動）。

例: テキストエディタでコード書き中に SDIT をクリックすると
ターミナル内の意図しない場所にクリックイベントが届く場合がある。

## 実装方針

1. `[mouse] swallow_mouse_click_on_focus = false` 設定を追加（デフォルト: false）
2. `true` のとき: `WindowEvent::Focused(true)` の直後に来た `MouseInput` イベントを1回だけ無視する
3. 実装: `SditApp` に `just_focused: bool` フラグを追加し、フォーカスイベント直後のクリックをスキップ

## 変更対象

- `crates/sdit-core/src/config/mod.rs` — `[mouse] swallow_mouse_click_on_focus` 追加
- `crates/sdit/src/app.rs` — `just_focused: bool` フィールド追加
- `crates/sdit/src/event_loop.rs` — Focused イベント後のクリックをスキップ

## セキュリティ影響

なし

## 参照

- WezTerm: `refs/wezterm/config/src/config.rs` `swallow_mouse_click_on_window_focus`
