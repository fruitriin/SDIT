# Phase 19.5: ウィンドウサブタイトル

**概要**: ウィンドウタイトルバーにサブタイトル（作業ディレクトリ等）を表示する機能を追加する。

**状態**: 完了

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| Config に subtitle 追加 | "none" / "working-directory" / "session-name" | sdit-core (`config/mod.rs`) | 完了 |
| CWD 変更時のタイトル更新 | OSC 7 受信時にウィンドウタイトルを更新 | sdit (`event_loop.rs`) | 完了 |
| テスト | 設定デシリアライズ | sdit-core | 完了 |

## 設定例

```toml
[window]
subtitle = "working-directory"  # "none" | "working-directory" | "session-name"
```

## 実装メモ

- `WindowSubtitle::WorkingDirectory` の場合、OSC 7 受信時に `"SDIT — ~/path"` 形式でタイトルを更新
- ホームディレクトリは `~` に省略して表示
- `WindowSubtitle::SessionName` は Config のみ追加（タイトル更新は将来実装）
- macOS の `NSWindow.subtitle` API は不要（winit の `set_title()` で代替）
- アクティブセッションのタイトルのみ更新（非アクティブセッションはスキップ）

## セキュリティ

- タイトルに設定するパスは OSC 7 経由でバリデーション済み（`parse_osc7_cwd` でサニタイズ）

## 参照

- `refs/ghostty/src/config/Config.zig` — window-subtitle
