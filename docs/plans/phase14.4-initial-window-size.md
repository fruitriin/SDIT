# Phase 14.4: 初期ウィンドウサイズ

**概要**: ウィンドウの初期サイズを列数×行数で指定可能にする。デフォルトの 80x24 を変更したいユーザー向け。

**状態**: 未着手

## 背景

- 現在のウィンドウサイズはデフォルトの winit ウィンドウサイズに依存
- ユーザーが常に大きな/小さなターミナルを好む場合、毎回リサイズが必要
- columns × rows の指定が最も直感的（ピクセル指定はフォントサイズに依存するため）

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| WindowConfig 拡張 | `window.columns` (u16, default: 80), `window.rows` (u16, default: 24) | sdit-core (`config/mod.rs`) | 未着手 |
| ウィンドウ生成 | columns × rows × cell_size からピクセルサイズを計算し、`with_inner_size` に渡す | sdit (`window_ops.rs`) | 未着手 |
| テスト | WindowConfig columns/rows serde 2件 + clamp 2件 | sdit-core | 未着手 |

## 設定例

```toml
[window]
columns = 120    # default: 80, range: 10-500
rows = 36        # default: 24, range: 2-200
```

## 注意事項

- padding が設定されている場合、padding 分をピクセルサイズに加算する（Phase 14.3 依存）
- Hot Reload では反映しない（既存ウィンドウのリサイズは混乱を招く）

## 依存関係

- Phase 14.3（ウィンドウパディング）— padding 考慮が必要。ただし padding=0 のデフォルトなら独立実装可能
