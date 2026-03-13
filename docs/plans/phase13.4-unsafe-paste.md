# Phase 13.4: Unsafe Paste 警告

**概要**: 改行を含むペースト時に確認ダイアログを表示するセキュリティ機能。

**状態**: 未着手

## タスク

| タスク | 詳細 | 変更先 |
|---|---|---|
| ペースト安全性判定 | 改行・制御文字を含むテキストの検出 | sdit (`event_loop.rs`) |
| 確認ダイアログ | macOS NSAlert で確認 | sdit (`event_loop.rs`) |
| 設定項目 | `paste.confirm_unsafe = true/false` | sdit-core (`config/`) |

## 参照

- `refs/ghostty/src/input/paste.zig`

## 依存関係

なし
