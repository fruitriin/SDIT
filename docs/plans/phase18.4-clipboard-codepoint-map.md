# Phase 18.4: クリップボード文字変換

**概要**: クリップボードにコピーする際に特定の Unicode 文字を別の文字に置換する機能を追加する。

**状態**: 完了

## 背景

- ボックス描画文字（U+2500 系）を他のアプリにペーストすると表示が崩れることがある
- コピー時に ASCII 互換文字に自動変換する機能があると便利

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| Config に clipboard_codepoint_map 設定追加 | HashMap<String, String> で Unicode 範囲→置換文字 | sdit-core (`config/mod.rs`) | 完了 |
| クリップボードコピー時の変換処理 | コピー文字列に対してマッピングを適用 | sdit (`action_handlers.rs`, `event_loop.rs`) | 完了 |
| テスト | 変換ロジックのユニットテスト | sdit-core | 完了 |

## セキュリティレビュー結果

| 重要度 | ID | 内容 | 対応 |
|---|---|---|---|
| Medium | M-1 | replacement 文字列無制限によるDoS | 修正済み: 256文字上限 + 出力10倍制限 |
| Medium | M-2 | パースフォーマット検証不足 | 修正済み: hex長・文字種バリデーション追加 |

## 設定例

```toml
[clipboard]
codepoint_map = { "U+2500-U+257F" = "-+|" }
```

## 参照

- `refs/ghostty/src/config/Config.zig` — clipboard-codepoint-map
