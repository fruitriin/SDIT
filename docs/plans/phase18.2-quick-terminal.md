# Phase 18.2: Quick Terminal

**概要**: グローバルホットキーで画面上端からスライドインするドロップダウンターミナルを実装する（macOS）。

**状態**: 未着手

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| Config に quick_terminal 設定追加 | enabled, position (top/bottom/left/right), size (0.0-1.0) | sdit-core (`config/mod.rs`) | 未着手 |
| グローバルホットキー登録 | macOS: CGEvent tap でシステム全体のホットキーを登録 | sdit (`quick_terminal.rs`) | 未着手 |
| Quick Terminal ウィンドウ | ボーダーレス、指定位置にスライドイン | sdit (`quick_terminal.rs`, `window_ops.rs`) | 未着手 |
| アニメーション | スライドイン/アウトのアニメーション | sdit (`quick_terminal.rs`) | 未着手 |
| テスト | 設定デシリアライズ | sdit-core | 未着手 |

## 設定例

```toml
[quick_terminal]
enabled = false
position = "top"      # top | bottom | left | right
size = 0.4            # 画面比率
hotkey = "ctrl+`"     # グローバルホットキー
```

## 参照

- `refs/ghostty/src/config/Config.zig` — quick-terminal-position, quick-terminal-size
