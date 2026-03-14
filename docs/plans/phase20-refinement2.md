# Phase 20: 品質・カスタマイズ向上バッチ 2

**概要**: リファレンス実装（Ghostty 等）との差分調査で発見した設定項目・VTE 完全性をバッチ実装する。

**状態**: 未着手

## サブフェーズ一覧

| Phase | 機能 | 概要 | 複雑度 |
|---|---|---|---|
| 20.1 | Bold as Bright + Faint Opacity | 太字→明色変換、暗字の透明度調整 | 低 |
| 20.2 | Window Position 保存 | ウィンドウ座標の保存・復帰 | 低 |
| 20.3 | Focus Follows Mouse | マウス乗り入れで自動フォーカス | 低 |
| 20.4 | OSC Color Report Format | カラーレポート形式の選択 | 低 |
| 20.5 | Title Report Flag | ウィンドウタイトル報告の許可/拒否 | 低 |
| 20.6 | Enquiry Response | ENQ (0x05) への応答カスタマイズ | 低 |
| 20.7 | Palette Generation | テーマカラーから自動配色生成 | 中 |
| 20.8 | Alpha Blending Mode | sRGB / Linear 切り替え | 中 |

## 参照

- `refs/ghostty/src/config/Config.zig`
- Phase 19 の設定追加パターンを踏襲
