# Phase 15.4: カスタムリンク + セッションリネーム

**概要**: カスタムリンク正規表現（URL 以外のクリック可能パターン）とセッション（タブ）のリネーム機能を実装する。

**状態**: 未着手

## 背景

- JIRA チケット番号やファイルパス:行番号などをクリックでブラウザやエディタで開く需要がある
- 複数セッション運用時、デフォルトのシェル名だけでは区別が困難

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| カスタムリンク設定 | 正規表現 + アクション（open URL template） | sdit-core (`config/mod.rs`) | 未着手 |
| カスタムリンク検出 | URL 検出パイプラインにカスタムパターンを統合 | sdit-core (`terminal/url_detector.rs`) | 未着手 |
| カスタムリンクアクション | Cmd+クリックでテンプレート URL を展開して open | sdit (`event_loop.rs`) | 未着手 |
| セッションリネーム | サイドバーでセッション名をダブルクリック→編集 | sdit (`event_loop.rs`, `render.rs`) | 未着手 |
| セッション名永続化 | session.toml にカスタム名を保存 | sdit-core (`session/`) | 未着手 |
| テスト | リンクパターン + テンプレート展開 + リネーム | sdit-core, sdit | 未着手 |

## 設定例

```toml
[[link]]
regex = "JIRA-\\d+"
action = "open:https://jira.example.com/browse/$0"

[[link]]
regex = "([\\w./]+):(\\d+)"
action = "open:vscode://file/$1:$2"
```

## 参照

- `refs/ghostty/src/config/Config.zig` — link, link-url
- `refs/zellij/zellij-utils/src/data.rs` — RenameTab
- `refs/wezterm/mux/src/tab.rs` — set_title

## 依存関係

- Phase 8.2（URL 検出）— リンク検出基盤
