# Phase 20.5: Title Report Flag

**概要**: アプリケーションからのウィンドウタイトル報答要求を許可/拒否する設定を追加する。

**状態**: 未着手

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| Config に title_report 追加 | bool (デフォルト false — セキュリティ上デフォルト拒否) | sdit-core (`config/mod.rs`) | 未着手 |
| CSI 21t (XTWINOPS) 応答の実装 | title_report が true の場合のみタイトルを応答 | sdit-core (`terminal/handler.rs`) | 未着手 |
| テスト | 設定デシリアライズ + 応答テスト | sdit-core | 未着手 |

## 設定例

```toml
[terminal]
title_report = false  # true でタイトル応答を許可
```

## 参照

- `refs/ghostty/src/config/Config.zig` — title-report
