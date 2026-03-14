# Phase 18.1: 背景画像

**概要**: ターミナルの背景に画像を描画する機能を追加する。

**状態**: 完了

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| Config に背景画像設定追加 | path, opacity (0.0-1.0), fit (contain/cover/fill) | sdit-core (`config/mod.rs`) | 完了 |
| 画像読み込み | image クレートで PNG/JPEG/WebP を読み込み、RGBA テクスチャに変換 | sdit (`window_ops.rs`) | 完了 |
| wgpu テクスチャ描画 | 背景レイヤーとして画像テクスチャを描画、opacity でブレンド | sdit-core (`render/pipeline.rs`), sdit (`render.rs`) | 完了 |
| Hot Reload 対応 | 設定変更時に画像を再読み込み | sdit (`event_loop.rs`) | 完了 |
| テスト | 設定デシリアライズ + 画像パスバリデーション | sdit-core | 完了 |

## セキュリティレビュー結果

| 重要度 | ID | 内容 | 対応 |
|---|---|---|---|
| Critical | C-1 | パストラバーサル + TOCTOU | 修正済み: canonicalize + ホームディレクトリ境界チェック + File::open→metadata 方式 |
| High | H-1 | wgpu テクスチャサイズ制限なし | 修正済み: validate_background_image_params() 切り出し、寸法/ピクセル数/バッファ長検証 |
| Low | L-1 | home_dir 失敗時 "." フォールバック | 修正済み: None を返す |
| Low | L-2 | 設定早期バリデーション未実装 | 記録のみ: エラーハンドリング自体は適切 |

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
