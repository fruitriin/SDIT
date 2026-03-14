# Phase 19.4: ダブルクリック判定間隔 + Grapheme 幅方式

**概要**: ダブルクリック判定の時間間隔を設定可能にし、Unicode grapheme の幅計算方式を選択可能にする。

**状態**: 未着手

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| Config に click_repeat_interval 追加 | u32 (ミリ秒、デフォルト 300) | sdit-core (`config/mod.rs`) | 未着手 |
| Config に grapheme_width_method 追加 | "unicode" / "legacy" (デフォルト unicode) | sdit-core (`config/mod.rs`) | 未着手 |
| マウスクリック判定で interval 設定を使用 | ダブル/トリプルクリックの判定閾値を設定値にする | sdit (`event_loop.rs`) | 未着手 |
| grapheme 幅計算の切り替え | unicode-width vs legacy wcswidth 互換 | sdit-core (`grid/mod.rs`) | 未着手 |
| テスト | 設定デシリアライズ + クランプ | sdit-core | 未着手 |

## 設定例

```toml
[mouse]
click_repeat_interval = 300  # ミリ秒

[terminal]
grapheme_width_method = "unicode"  # "unicode" | "legacy"
```

## 参照

- `refs/ghostty/src/config/Config.zig` — click-repeat-interval, grapheme-width-method
