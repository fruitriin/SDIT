# Phase 19.5: ウィンドウサブタイトル

**概要**: ウィンドウタイトルバーにサブタイトル（作業ディレクトリ等）を表示する機能を追加する。

**状態**: 未着手

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| Config に window_subtitle 追加 | "none" / "working-directory" / "session-name" | sdit-core (`config/mod.rs`) | 未着手 |
| macOS タイトルバーのサブタイトル設定 | NSWindow.subtitle API (macOS 11+) | sdit (`window_ops.rs`) | 未着手 |
| CWD 変更時のサブタイトル更新 | OSC 7 (working directory report) 受信時に更新 | sdit (`event_loop.rs`) | 未着手 |
| テスト | 設定デシリアライズ | sdit-core | 未着手 |

## 設定例

```toml
[window]
subtitle = "working-directory"  # "none" | "working-directory" | "session-name"
```

## 参照

- `refs/ghostty/src/config/Config.zig` — window-subtitle
