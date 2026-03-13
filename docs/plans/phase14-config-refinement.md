# Phase 14: 設定精緻化・操作性向上

**概要**: 日常使用の快適性を高める設定項目の追加と、上級ユーザー向けの操作機能を実装する。

各サブフェーズは独立ファイルに分割:

| Phase | ファイル | 状態 |
|---|---|---|
| 14.1 | `phase14.1-cursor-config.md` | 未着手 |
| 14.2 | `phase14.2-scrollback-config.md` | 未着手 |
| 14.3 | `phase14.3-window-padding.md` | 未着手 |
| 14.4 | `phase14.4-initial-window-size.md` | 未着手 |
| 14.5 | `phase14.5-shell-integration.md` | 未着手 |
| 14.6 | `phase14.6-quick-select.md` | 未着手 |

## 実装順序の推奨

1. Phase 14.1（カーソル設定）— 最も視認性に影響、DECSCUSR 基盤は実装済み
2. Phase 14.2（スクロールバック設定）— ハードコード値の設定化、低コスト
3. Phase 14.3（ウィンドウパディング）— 見た目の完成度向上
4. Phase 14.4（初期ウィンドウサイズ）— ユーザー環境への適応
5. Phase 14.5（シェルインテグレーション）— 上級機能の基盤
6. Phase 14.6（Quick Select）— 上級ユーザー向け生産性機能
