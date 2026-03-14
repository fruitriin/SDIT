# Phase 17: 品質・UX 向上（第2弾）

**概要**: Phase 16 に続き、リファレンス調査で発見された当たり前品質・あったら便利な機能を実装する。

**状態**: 未着手

## サブフェーズ一覧

| Phase | 機能 | 分類 | 規模 | 優先度 |
|---|---|---|---|---|
| 17.1 | テーマプリセット | 当たり前品質 | 中 | 高 |
| 17.2 | ウィンドウデコレーション設定 | あったら便利 | 小 | 中高 |
| 17.3 | Always On Top | あったら便利 | 小 | 中高 |
| 17.4 | 右クリック動作カスタマイズ | あったら便利 | 小 | 中 |
| 17.5 | コマンドパレット | あったら便利 | 大 | 中 |
| 17.6 | セッション復帰 | 当たり前品質 | 大 | 中 |

## 参照

- `refs/ghostty/src/config/Config.zig` — テーマ、ウィンドウデコレーション、always-on-top
- `refs/ghostty/src/apprt/action.zig` — toggle_window_decorations, float_window, toggle_command_palette
- `refs/wezterm/wezterm-gui/src/termwindow/palette.rs` — コマンドパレット
- `refs/alacritty/alacritty/src/config/window.rs` — decorations
