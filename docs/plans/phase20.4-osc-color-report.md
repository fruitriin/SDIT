# Phase 20.4: OSC Color Report Format

**概要**: OSC 10/11 等のカラー問い合わせ応答の形式を設定可能にする。

**状態**: 完了

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| Config に osc_color_report_format 追加 | "8-bit" / "16-bit" (デフォルト 16-bit) | sdit-core (`config/mod.rs`) | 完了 |
| OSC 10/11 応答の実装 | カラー問い合わせに応答する | sdit-core (`terminal/mod.rs`) | 完了 |
| テスト | 設定デシリアライズ + 応答フォーマット | sdit-core | 完了 |

## 設定例

```toml
[terminal]
osc_color_report_format = "16-bit"  # "8-bit" | "16-bit"
```

## セキュリティレビュー結果

| 重要度 | ID | 内容 | 対応 |
|---|---|---|---|
| Low | L-4 | OSC 10/11 応答が固定値（実際のカラー値ではない） | 記録のみ: 将来対応 |

## 参照

- `refs/ghostty/src/config/Config.zig` — osc-color-report-format
