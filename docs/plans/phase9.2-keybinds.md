# Phase 9.2: キーバインドカスタマイズ

**概要**: TOML設定ファイルからキーバインドを定義・変更可能にする。

## タスク

| タスク | 詳細 | 変更先クレート |
|---|---|---|
| キーバインド設定スキーマ | TOML `[keybinds]` セクション定義 | sdit-config |
| アクション列挙型 | `NewWindow`, `NewTab`, `CloseTab`, `Copy`, `Paste`, `Search` 等をenum化 | sdit-config |
| ショートカット判定リファクタリング | `is_*_shortcut()` 関数群を設定駆動に置換 | sdit (`main.rs`) |

## 依存関係

Phase 8

## リファレンス

- `refs/alacritty/alacritty/src/config/bindings.rs` — キーバインド設定の型定義
