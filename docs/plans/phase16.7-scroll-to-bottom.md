# Phase 16.7: スクロールトゥボトム設定

**概要**: キー入力時や新しい出力があったときに自動でスクロールバックの末尾に戻るかどうかの設定を追加する。

**状態**: 未着手

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| ScrollingConfig に scroll_to_bottom 追加 | `on_keystroke: bool`（デフォルト true）、`on_output: bool`（デフォルト false） | sdit-core (`config/mod.rs`) | 未着手 |
| キー入力時の自動スクロール | display_offset > 0 のとき自動で 0 に戻す | sdit (`event_loop.rs`) | 未着手 |
| 出力時の自動スクロール | PTY 出力受信時に display_offset を 0 に戻す | sdit (`event_loop.rs`) | 未着手 |
| テスト | 設定デシリアライズ | sdit-core | 未着手 |

## 設定例

```toml
[scrolling]
scroll_to_bottom_on_keystroke = true
scroll_to_bottom_on_output = false
```

## 参照

- `refs/ghostty/src/config/Config.zig` — scroll-to-bottom
