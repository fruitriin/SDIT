# Phase 20.6: Enquiry Response

**概要**: ENQ (0x05) への応答文字列をカスタマイズ可能にする。

**状態**: 未着手

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| Config に enquiry_response 追加 | Option<String> (デフォルト None = 空応答) | sdit-core (`config/mod.rs`) | 未着手 |
| ENQ 受信時の応答 | execute() の 0x05 で pending_writes に応答文字列を追加 | sdit-core (`terminal/mod.rs`) | 未着手 |
| テスト | 設定デシリアライズ + ENQ 応答 | sdit-core | 未着手 |

## 設定例

```toml
[terminal]
enquiry_response = ""  # 空文字列 = 応答しない
```

## 参照

- `refs/ghostty/src/config/Config.zig` — enquiry-response
