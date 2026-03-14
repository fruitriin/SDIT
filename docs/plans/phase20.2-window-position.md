# Phase 20.2: ウィンドウ座標保存

**概要**: ウィンドウの初期位置を設定可能にする。

**状態**: 完了

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| Config に position_x/y 追加 | Option<i32> (デフォルト None) | sdit-core (`config/mod.rs`) | 完了 |
| clamped_position() メソッド | -16000〜32000 にクランプ | sdit-core (`config/mod.rs`) | 完了 |
| ウィンドウ生成時に位置を設定 | winit の with_position() | sdit (`window_ops.rs`) | 完了 |
| テスト | 設定デシリアライズ + クランプ | sdit-core | 完了 |

## 設定例

```toml
[window]
position_x = 100
position_y = 200
```

## セキュリティレビュー結果

| 重要度 | ID | 内容 | 対応 |
|---|---|---|---|
| Medium | M-1 | 画面外配置の危険性 | 修正済み: clamped_position() で -16000〜32000 にクランプ |

## 参照

- `refs/ghostty/src/config/Config.zig` — window-position-x, window-position-y
