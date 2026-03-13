# Phase 13: 当たり前品質向上

**概要**: 日常使用で「ないと乗り換えがつらい」機能を追加し、SDIT を常用可能な品質にする。

各サブフェーズは独立ファイルに分割済み:

| Phase | ファイル | 状態 |
|---|---|---|
| 13.1 | `phase13.1-option-as-alt.md` | **完了** |
| 13.2 | `phase13.2-visual-bell.md` | **完了** |
| 13.3 | `phase13.3-window-opacity.md` | **完了** |
| 13.4 | `phase13.4-unsafe-paste.md` | **完了** |
| 13.5 | `phase13.5-kitty-keyboard.md` | **完了** |
| 13.6 | `phase13.6-desktop-notification.md` | **完了** |

## 実装順序の推奨

1. Phase 13.1（Option as Alt）— 最も低コスト・高インパクト
2. Phase 13.2（ビジュアルベル）— 低コスト
3. Phase 13.4（Unsafe Paste 警告）— セキュリティ
4. Phase 13.3（背景透過）— ユーザー人気
5. Phase 13.5（Kitty Keyboard Protocol）— neovim ユーザー向け
6. Phase 13.6（デスクトップ通知）— 利便性
