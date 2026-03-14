# Phase 17.4: 右クリック動作カスタマイズ

**概要**: 右クリック時の動作を設定で変更可能にする（コンテキストメニュー / ペースト / 無効）。

**状態**: **完了**

## セキュリティレビュー結果（Phase 17.1〜17.4 共通）

| 重要度 | ID | 概要 | 対応 |
|---|---|---|---|
| Low | L-1 | ThemeName::all() の位置独立性 | テストで検証済み |
| Low | L-2 | テーマ設定保存エラーのログレベル | warn で記録済み |
| Low | L-3 | enum デシリアライズのフォールバック | serde デフォルト動作で十分 |
| Low | L-4 | Paste ロジックの重複 | 将来リファクタリング候補 |
| Info | I-1 | ThemeName に Copy derive 推奨 | 最適化候補 |
| Info | I-2 | enum バリアント追加時の match 漏れ | Rust 型安全性で保障 |
| Info | I-3 | テーマコントラスト検証の完全性 | テスト完備 |

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| Config に right_click_action 設定追加 | `context_menu`（デフォルト）/ `paste` / `none` | sdit-core (`config/mod.rs`) | 未着手 |
| 右クリックハンドラー変更 | 設定に応じてコンテキストメニュー表示 or ペースト or 何もしない | sdit (`event_loop.rs`) | 未着手 |
| テスト | 設定デシリアライズ | sdit-core | 未着手 |

## 設定例

```toml
[mouse]
right_click_action = "context_menu"  # context_menu | paste | none
```

## 参照

- `refs/ghostty/src/config/Config.zig` — right-click
