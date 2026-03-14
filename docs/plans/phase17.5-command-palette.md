# Phase 17.5: コマンドパレット

**概要**: ターミナル内にコマンドパレット UI を表示し、キーバインド可能なアクションをファジー検索して実行する。

**状態**: **完了**

## 背景

- VS Code スタイルのコマンドパレットは多機能アプリケーションの標準的な UX
- SDIT のアクション数が増えてきたため、ファジー検索による発見性向上が有用

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| アクション一覧取得 | 全 Action バリアントの名前・説明リストを生成 | sdit-core (`config/keybinds.rs`) | 未着手 |
| ファジー検索エンジン | 入力文字列でアクションリストをフィルタリング・スコアリング | sdit (`command_palette.rs`) | 未着手 |
| パレット UI 描画 | 中央オーバーレイ: テキスト入力 + 候補リスト | sdit (`render.rs`, `command_palette.rs`) | 未着手 |
| キー入力処理 | パレットモード中のテキスト入力・選択・実行・ESC キャンセル | sdit (`event_loop.rs`) | 未着手 |
| Action::ToggleCommandPalette | Cmd+Shift+P でパレットを開閉 | sdit-core (`config/keybinds.rs`) | 未着手 |
| テスト | ファジー検索ロジック | sdit-core or sdit | 未着手 |

## 設定例

```toml
[keybind]
"super+shift+p" = "ToggleCommandPalette"
```

## 参照

- `refs/ghostty/src/apprt/action.zig` — toggle_command_palette
- `refs/wezterm/wezterm-gui/src/termwindow/palette.rs` — CommandPalette
