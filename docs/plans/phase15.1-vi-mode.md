# Phase 15.1: vi モード（コピーモード）

**概要**: キーボードのみでスクロールバック内を移動・選択・ヤンクできるモード。hjkl/w/b/0/$/{/} の vi 風モーションと行選択・単語選択・ブロック選択を実装する。

**状態**: **完了**

## 背景

- vim/nvim ユーザーがターミナルを乗り換えるとき、vi モードの有無は最大の判断材料のひとつ
- Alacritty は `ToggleViMode` アクションで完全な vi モードを持つ
- 既存の検索機能（Phase 9.1）と統合し、`/` で前方検索、`?` で後方検索を起動可能にする

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| ViMode 状態管理 | ViModeState（カーソル位置・選択範囲・モード） | sdit (`app.rs`) | **完了** |
| vi モーション | h/j/k/l/w/b/e/0/$/{/}/gg/G/H/M/L | sdit-core (`terminal/vi_mode.rs`) | **完了** |
| 選択モード | v（文字選択）、V（行選択）、Ctrl+v（ブロック選択） | sdit-core | **完了** |
| ヤンク | y でクリップボードにコピー | sdit (`vi_mode.rs`) | **完了** |
| 検索統合 | /（前方検索）、n/N（次/前の結果） | sdit (`vi_mode.rs`) | **完了** |
| vi カーソル描画 | ブロックカーソルの描画（通常カーソルと区別） | sdit (`render.rs`) | **完了** |
| テスト | モーション 12件 unit テスト | sdit-core | **完了** |

## セキュリティレビュー結果

| 重要度 | ID | 概要 | 対応 |
|---|---|---|---|
| Medium | M-1 | 選択座標がスクロール時にサイレントにずれる | **修正済み** — display_offset 変換を除去、グリッド絶対座標で Selection に渡すよう変更 |
| Medium | M-2 | リサイズ時に vi カーソル座標が未検証 | **修正済み** — handle_resize / apply_config_reload で vi_mode をリセット |
| Low | L-1 | history の usize→i32 キャストオーバーフロー | config でスクロールバック上限がクランプ済みのため実害なし |
| Low | L-2 | word_right の min_line 不使用 | コードの臭いだが機能上の問題なし |
| Low | L-3 | view_bottom が display_offset >= screen_lines 時に負値 | 視覚的な表示ズレにとどまる |
| Low | L-4 | handle_vi_mode_key が vi モード終了直後も true を返す | 呼び出し側でガード済み |
| Info | I-1 | Block 選択が SelectionType::Simple に紛れるリスク | 将来実装時に対応 |
| Info | I-2 | is_empty_line の段落移動ループ計算量 | 低リスク |
| Info | I-3 | grid_cell_at の usize 加算オーバーフロー | raw.len() チェックで弾かれるため低リスク |

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
