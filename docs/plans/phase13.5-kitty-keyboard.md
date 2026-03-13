# Phase 13.5: Kitty Keyboard Protocol

**概要**: neovim 等が要求する拡張キーボードプロトコル。修飾キーの正確な報告、キーリリースイベント等を提供。

**状態**: 未着手

## タスク

| タスク | 詳細 | 変更先 |
|---|---|---|
| CSI u エンコーディング | Kitty progressive enhancement flags 対応 | sdit (`input.rs`) |
| モード管理 | `CSI > Ps u` (push) / `CSI < u` (pop) | sdit-core (`terminal/`) |
| modifyOtherKeys | xterm mode 2 互換 | sdit (`input.rs`) |

## 参照

- `refs/ghostty/src/input/key_encode.zig`
- `refs/ghostty/src/input/kitty.zig`

## 依存関係

なし
