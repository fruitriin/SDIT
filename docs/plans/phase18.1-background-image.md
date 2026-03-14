# Phase 18.1: 背景画像

**概要**: ターミナルの背景に画像を描画する機能を追加する。

**状態**: 未着手

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| Config に背景画像設定追加 | path, opacity (0.0-1.0), fit (contain/cover/fill/none) | sdit-core (`config/mod.rs`) | 未着手 |
| 画像読み込み | image クレートで PNG/JPEG/WebP を読み込み、RGBA テクスチャに変換 | sdit (`render.rs`) | 未着手 |
| wgpu テクスチャ描画 | 背景レイヤーとして画像テクスチャを描画、opacity でブレンド | sdit (`render.rs`) | 未着手 |
| Hot Reload 対応 | 設定変更時に画像を再読み込み | sdit (`event_loop.rs`) | 未着手 |
| テスト | 設定デシリアライズ + 画像パスバリデーション | sdit-core | 未着手 |

## 設定例

```toml
[window]
background_image = "~/.config/sdit/bg.png"
background_image_opacity = 0.3
background_image_fit = "cover"  # contain | cover | fill | none
```

## 依存クレート候補

- `image` クレート（画像デコード）— 既に広く使われている

## 参照

- `refs/ghostty/src/config/Config.zig` — background-image, background-image-opacity, background-image-fit
