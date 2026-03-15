# Phase 26.3: グローバルホットキーの追加アクション設定（macOS）

## 要望

現在 SDIT は Quick Terminal 専用のグローバルホットキー（`global_hotkey`）のみ対応している。
任意のアクションをグローバルホットキーに割り当てられるよう拡張する。

Ghostty: `global:` プレフィックス付きキーバインド（Config.zig:1698）

## ユースケース

- `Cmd+Shift+Alt+T`: SDIT をフォアグラウンドに持ってくる（HideShow）
- `Cmd+Shift+Alt+N`: どこからでも新しいウィンドウを開く
- `Cmd+Shift+Alt+C`: クリップボードの内容をどこからでもコピー

## 実装方針

1. キーバインド設定に `"global:action_name"` 構文を追加（macOS のみ）
2. `global-hotkey` クレートで OS レベルのキーバインドを登録
3. マルチグローバルホットキー対応（現在は 1 つのみ）

```toml
[keybinds]
global_hotkeys = [
  { key = "cmd+shift+alt+t", action = "bring_to_front" },
  { key = "cmd+shift+alt+n", action = "new_window" },
]
```

### 追加アクション

- `bring_to_front`: SDIT の全ウィンドウをフォアグラウンドに持ってくる

## 変更対象

- `crates/sdit-core/src/config/mod.rs` — `global_hotkeys: Vec<GlobalHotkeyBinding>` 設定追加
- `crates/sdit/src/app.rs` — 複数グローバルホットキーの登録・管理

## 実装結果（2026-03-15 完了）

- `GlobalHotkeyBinding { hotkey, action }` を keybinds.rs に追加
- `Config.global_hotkeys: Vec<GlobalHotkeyBinding>` を追加
- `Action::BringToFront` を追加（全ウィンドウをフォアグラウンドに）
- `SditEvent::GlobalHotkeyAction(Action)` を追加し MenuAction ハンドラに合流
- `register_user_global_hotkeys()` ヘルパーで hotkey 登録 + 専用スレッド起動
- テスト: 448 件 PASS

使用例:
```toml
[[global_hotkeys]]
hotkey = "cmd+shift+alt+t"
action = "BringToFront"
```

## セキュリティ影響

macOS のアクセシビリティ権限が必要（既存の Quick Terminal と同様）。

## 参照

- Ghostty: `refs/ghostty/src/config/Config.zig` L1698 `global:` prefix
- global-hotkey クレート（既に依存済み）
