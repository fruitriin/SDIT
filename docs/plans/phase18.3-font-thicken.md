# Phase 18.3: フォント太さ調整

**概要**: macOS でフォントのレンダリング時に線を太くする機能を追加する。

**状態**: 未着手

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| Config に font_thicken 設定追加 | bool (デフォルト false) | sdit-core (`config/font.rs`) | 未着手 |
| cosmic-text レンダリング調整 | SwashCache のグリフビットマップに対して太字化処理 | sdit-core (`render/font.rs`) | 未着手 |
| テスト | 設定デシリアライズ | sdit-core | 未着手 |

## 設定例

```toml
[font]
thicken = false  # macOS のみ有効
```

## 参照

- `refs/ghostty/src/config/Config.zig` — font-thicken
