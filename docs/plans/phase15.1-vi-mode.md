# Phase 15.1: vi モード（コピーモード）

**概要**: キーボードのみでスクロールバック内を移動・選択・ヤンクできるモード。hjkl/w/b/0/$/{/} の vi 風モーションと行選択・単語選択・ブロック選択を実装する。

**状態**: 未着手

## 背景

- vim/nvim ユーザーがターミナルを乗り換えるとき、vi モードの有無は最大の判断材料のひとつ
- Alacritty は `ToggleViMode` アクションで完全な vi モードを持つ
- 既存の検索機能（Phase 9.1）と統合し、`/` で前方検索、`?` で後方検索を起動可能にする

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| ViMode 状態管理 | ViModeState（カーソル位置・選択範囲・モード） | sdit (`app.rs`) | 未着手 |
| vi モーション | h/j/k/l/w/b/e/0/$/{/}/gg/G/H/M/L | sdit-core (`terminal/vi_mode.rs`) | 未着手 |
| 選択モード | v（文字選択）、V（行選択）、Ctrl+v（ブロック選択） | sdit-core | 未着手 |
| ヤンク | y でクリップボードにコピー | sdit (`event_loop.rs`) | 未着手 |
| 検索統合 | /（前方検索）、?（後方検索）、n/N（次/前の結果） | sdit (`event_loop.rs`) | 未着手 |
| vi カーソル描画 | ブロックカーソルの描画（通常カーソルと区別） | sdit (`render.rs`) | 未着手 |
| テスト | モーション 8件 + 選択 3件 + ヤンク 2件 | sdit-core, sdit | 未着手 |

## 設定例

```toml
[keybinds.macos]
"Cmd+Shift+V" = "ToggleViMode"
```

## 参照

- `refs/alacritty/alacritty/src/config/bindings.rs` — ViAction, ViMotion の定義
- `refs/alacritty/alacritty_terminal/src/vi_mode.rs` — vi モード実装

## 依存関係

- Phase 6.2（テキスト選択 + クリップボード）— 選択基盤
- Phase 9.1（検索）— 検索統合
