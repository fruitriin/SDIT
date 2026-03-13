# Phase 14.3: ウィンドウパディング

**概要**: ターミナルグリッドとウィンドウ端の間に余白（パディング）を設定可能にする。テキストがウィンドウ枠に密着しない、視認性の高い表示を実現する。

**状態**: **完了**

## 背景

- 現在、テキストはウィンドウの左上隅から直接描画される
- Alacritty/Ghostty はパディング設定を持ち、多くのユーザーが利用している
- パディングはグリッドサイズ計算・カーソル位置・マウスクリック座標すべてに影響する

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| WindowConfig 拡張 | `window.padding_x`, `window.padding_y` (u16, default: 0, clamp: 0-200) | sdit-core (`config/mod.rs`) | **完了** |
| グリッドサイズ計算 | パディング分を差し引いた有効領域でグリッドサイズを算出 | sdit (`window.rs`, `render.rs`) | **完了** |
| 描画オフセット | セル描画時に padding_x/padding_y をオフセットとして加算 | sdit (`render.rs`) | **完了** |
| マウス座標補正 | クリック・スクロール座標からパディングを差し引く | sdit (`input.rs`) | **完了** |
| カーソル描画補正 | カーソル位置にパディングオフセットを適用 | sdit (`render.rs`) | **完了** |
| Hot Reload 対応 | 設定変更時にグリッドサイズ再計算 + PTY リサイズ | sdit (`event_loop.rs`) | **完了** |
| テスト | WindowConfig padding serde 2件 + clamp 2件 | sdit-core | **完了** |

## 設定例

```toml
[window]
padding_x = 8    # pixels, default: 0
padding_y = 4    # pixels, default: 0
```

## 参照

- `refs/alacritty/alacritty/src/display/mod.rs` — SizeInfo padding_x/padding_y

## 依存関係

なし
