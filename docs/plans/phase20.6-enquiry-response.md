# Phase 20.6: Enquiry Response

**概要**: ENQ (0x05) への応答文字列をカスタマイズ可能にする。

**状態**: 完了

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| Config に enquiry_response 追加 | Option<String> (デフォルト None) | sdit-core (`config/mod.rs`) | 完了 |
| clamped_enquiry_response() | 最大256文字にクランプ | sdit-core (`config/mod.rs`) | 完了 |
| ENQ 受信時の応答 | execute() 0x05 で pending_writes に追加 | sdit-core (`terminal/mod.rs`) | 完了 |
| テスト | 設定デシリアライズ + ENQ 応答 + クランプ | sdit-core | 完了 |

## 設定例

```toml
[terminal]
enquiry_response = ""  # None/未設定 = 応答しない、最大256文字
```

## セキュリティレビュー結果

| 重要度 | ID | 内容 | 対応 |
|---|---|---|---|
| Medium | M-3 | sdit-core レイヤーでの検証なし | 修正済み: clamped_enquiry_response() を config/mod.rs に追加 |

## 参照

- `refs/ghostty/src/config/Config.zig` — enquiry-response
