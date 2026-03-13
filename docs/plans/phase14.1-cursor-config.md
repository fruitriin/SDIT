# Phase 14.1: カーソル設定

**概要**: デフォルトのカーソルスタイル・点滅有無・カーソル色を設定可能にする。アプリケーション（neovim 等）が DECSCUSR でスタイルを変更した後、終了時にデフォルトに戻せるようにする。

**状態**: 完了

## 背景

- `CursorStyle`（Block/Underline/Bar）と `cursor_blinking` は Terminal に既に存在する
- DECSCUSR ハンドラも実装済み（handler.rs）
- 現在のデフォルトは Block + 非点滅にハードコード
- ユーザーが好みのカーソルを設定できない
- アプリケーション終了時に DECSCUSR 0（デフォルトに戻す）を受信したとき、設定値に戻す必要がある

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| CursorConfig 追加 | `cursor.style` (block/underline/bar), `cursor.blinking` (bool), `cursor.color` (optional hex) | sdit-core (`config/mod.rs`) | **完了** |
| Terminal 初期化 | CursorConfig から初期スタイル・点滅を設定 | sdit-core (`terminal/mod.rs`) | **完了** |
| DECSCUSR 0 ハンドラ | パラメータ 0 で設定のデフォルトスタイルに復帰 | sdit-core (`terminal/handler.rs`) | **完了** |
| カーソル色描画 | CursorConfig.color が設定されていれば、カーソル描画時にその色を使用 | sdit (`render.rs`) | **完了** |
| Hot Reload 対応 | 設定変更時に既存 Terminal のデフォルト値を更新 | sdit (`event_loop.rs`) | **完了** |
| テスト | CursorConfig serde 5件 + DECSCUSR 0 リセット 1件 + new_with_cursor 1件 + parse_hex_color 3件 | sdit-core / sdit | **完了** |

## 設定例

```toml
[cursor]
style = "bar"         # "block" (default), "underline", "bar"
blinking = true       # default: false
color = "#ff6600"     # optional, default: theme foreground color
```

## 参照

- `refs/alacritty/alacritty/src/config/cursor.rs` — カーソル設定構造
- `refs/ghostty/src/terminal/cursor.zig` — カーソルスタイル管理

## 依存関係

なし
