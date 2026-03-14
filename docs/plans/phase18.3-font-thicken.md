# Phase 18.3: フォント太さ調整

**概要**: macOS でフォントのレンダリング時に線を太くする機能を追加する。

**状態**: 完了

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| Config に font_thicken 設定追加 | bool (デフォルト false) | sdit-core (`config/font.rs`) | 完了 |
| cosmic-text レンダリング調整 | SwashCache のグリフビットマップに対して alpha 増幅 (×1.6) | sdit-core (`render/font.rs`) | 完了 |
| テスト | 設定デシリアライズ | sdit-core | 完了 |

## セキュリティレビュー結果

| 重要度 | ID | 内容 | 対応 |
|---|---|---|---|
| Medium | M-3 | 数値演算コメント不明確 | 修正済み: >> 7 → / 128 + コメント改善 |

## 設定例

```toml
[font]
thicken = false  # macOS のみ有効
```

## 参照

- `refs/ghostty/src/config/Config.zig` — font-thicken
