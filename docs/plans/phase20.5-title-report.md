# Phase 20.5: Title Report Flag

**概要**: アプリケーションからのウィンドウタイトル報答要求を許可/拒否する設定を追加する。

**状態**: 完了

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| Config に title_report 追加 | bool (デフォルト false) | sdit-core (`config/mod.rs`) | 完了 |
| CSI 21t (XTWINOPS) 応答 | title_report=true の場合のみ応答 | sdit-core (`terminal/handler.rs`) | 完了 |
| テスト | 設定デシリアライズ + 応答テスト | sdit-core | 完了 |

## 設定例

```toml
[terminal]
title_report = false  # true でタイトル応答を許可
```

## セキュリティレビュー結果

| 重要度 | ID | 内容 | 対応 |
|---|---|---|---|
| High | H-1 | CSI 21t 応答のバッファ超過 | 修正済み: タイトルをトランケートして MAX_PENDING_WRITES 以内に |
| Medium | M-2 | バッファ破棄時のログ出力なし | 修正済み: write_response() に log::warn! 追加 |

## 参照

- `refs/ghostty/src/config/Config.zig` — title-report
