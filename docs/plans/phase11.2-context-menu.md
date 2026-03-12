# Phase 11.2: 右クリックコンテキストメニュー

**概要**: ターミナル領域とサイドバー領域で右クリックメニューを表示する。

## タスク

| タスク | 詳細 | 変更先クレート |
|---|---|---|
| MouseButton::Right イベント処理 | 右クリック検出（現在は Left のみ実装） | sdit (`main.rs`) |
| ターミナル領域メニュー | Copy, Paste, Select All, Search | sdit |
| サイドバー領域メニュー | Close Session, Detach to New Window, Rename | sdit |
| muda PopupMenu 統合 | ネイティブコンテキストメニューを muda で表示 | sdit |

## 依存関係

- Phase 11.1（muda クレートの導入）
- Phase 6.2（Copy/Paste の実装）
