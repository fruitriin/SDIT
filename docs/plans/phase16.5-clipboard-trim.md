# Phase 16.5: クリップボードコピー時末尾空白削除

**概要**: テキスト選択→コピー時に行末の空白を自動削除するオプションを追加する。

**状態**: **完了**

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| SelectionConfig に trim_trailing_spaces 追加 | デフォルト true | sdit-core (`config/mod.rs`) | **完了** |
| コピー処理で適用 | クリップボード書き込み前に各行の末尾空白を削除 | sdit (`event_loop.rs`) | **完了** |
| テスト | 設定デシリアライズ | sdit-core | **完了** |

## 設定例

```toml
[selection]
trim_trailing_spaces = true
```

## 参照

- `refs/ghostty/src/config/Config.zig` — clipboard-trim-trailing-spaces
