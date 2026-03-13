# Phase 15.2: 入力・選択の精緻化

**概要**: スクロール倍率設定、セマンティック単語境界、タイピング中マウスカーソル非表示、選択時クリップボード自動コピーをまとめて実装する。

**状態**: 未着手

## 背景

- スクロールが速すぎ/遅すぎはすぐ不満になる（当たり前品質）
- ダブルクリック単語選択の境界文字設定は開発者向けターミナルとして必須
- hide_when_typing と save_to_clipboard はオプション設定として採用価値が高い

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| scrolling.multiplier | マウスホイール1ノッチあたりのスクロール行数（デフォルト 3） | sdit-core (`config/mod.rs`), sdit (`event_loop.rs`) | 未着手 |
| selection-word-chars | ダブルクリック単語選択の境界文字カスタマイズ | sdit-core (`config/mod.rs`, `terminal/mod.rs`) | 未着手 |
| hide_when_typing | キー入力中マウスカーソルを非表示（デフォルト false） | sdit (`event_loop.rs`) | 未着手 |
| save_to_clipboard | テキスト選択と同時にクリップボードにコピー（デフォルト false） | sdit (`event_loop.rs`) | 未着手 |
| テスト | 各設定のバリデーション + 単語境界テスト | sdit-core | 未着手 |

## 設定例

```toml
[scrolling]
multiplier = 3   # default: 3

[selection]
word_chars = ""  # 空=デフォルト区切り
save_to_clipboard = false

[mouse]
hide_when_typing = false
```

## 参照

- `refs/alacritty/alacritty/src/config/scrolling.rs` — scrolling.multiplier
- `refs/alacritty/alacritty/src/config/mouse.rs` — hide_when_typing
- `refs/alacritty/alacritty/src/config/selection.rs` — save_to_clipboard
- `refs/ghostty/src/config/Config.zig` — selection-word-chars

## 依存関係

なし
