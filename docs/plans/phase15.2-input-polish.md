# Phase 15.2: 入力・選択の精緻化

**概要**: スクロール倍率設定、セマンティック単語境界、タイピング中マウスカーソル非表示、選択時クリップボード自動コピーをまとめて実装する。

**状態**: **完了**

## 背景

- スクロールが速すぎ/遅すぎはすぐ不満になる（当たり前品質）
- ダブルクリック単語選択の境界文字設定は開発者向けターミナルとして必須
- hide_when_typing と save_to_clipboard はオプション設定として採用価値が高い

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| scrolling.multiplier | マウスホイール1ノッチあたりのスクロール行数（デフォルト 3） | sdit-core (`config/mod.rs`), sdit (`event_loop.rs`) | **完了** |
| selection-word-chars | ダブルクリック単語選択の境界文字カスタマイズ | sdit-core (`config/mod.rs`, `terminal/mod.rs`) | **完了** |
| hide_when_typing | キー入力中マウスカーソルを非表示（デフォルト false） | sdit (`event_loop.rs`) | **完了** |
| save_to_clipboard | テキスト選択と同時にクリップボードにコピー（デフォルト false） | sdit (`event_loop.rs`) | **完了** |
| テスト | 各設定のバリデーション + 単語境界テスト | sdit-core | **完了** |

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

## セキュリティレビュー結果

| 重要度 | ID | 概要 | 対応 |
|---|---|---|---|
| Medium | M-1 | LineDelta * multiplier の整数オーバーフロー | **修正済み** — saturating_mul に変更 |
| Low | L-1 | cursor_hidden フラグの状態不整合（ウィンドウ消滅時） | 実害は低い |
| Low | L-2 | cursor_hidden がウィンドウ横断で共有 | SDI環境での軽微な問題 |
| Low | L-3 | word_chars に制御文字が含まれうる | 実害は低い |
| Low | L-4 | save_to_clipboard のサイズ上限なし | 実害は低い |
| Info | I-1 | multiplier/word_chars フィールドの pub 公開 | 設計検討 |
| Info | I-2 | LineDelta の NaN/inf は Rust 1.45+ で安全 | 確認済み |

## 依存関係

なし
