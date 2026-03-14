# Phase 15.4: カスタムリンク + セッションリネーム

**概要**: カスタムリンク正規表現（URL 以外のクリック可能パターン）とセッション（タブ）のリネーム機能を実装する。

**状態**: **完了**

## 背景

- JIRA チケット番号やファイルパス:行番号などをクリックでブラウザやエディタで開く需要がある
- 複数セッション運用時、デフォルトのシェル名だけでは区別が困難

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| カスタムリンク設定 | 正規表現 + アクション（open URL template） | sdit-core (`config/mod.rs`) | **完了** |
| カスタムリンク検出 | URL 検出パイプラインにカスタムパターンを統合 | sdit-core (`terminal/url_detector.rs`) | **完了** |
| カスタムリンクアクション | Cmd+クリックでテンプレート URL を展開して open | sdit (`event_loop.rs`) | **完了** |
| セッションリネーム | サイドバーでセッション名をダブルクリック→編集 | sdit (`event_loop.rs`, `render.rs`) | **完了** |
| セッション名永続化 | session.toml にカスタム名を保存 | sdit-core (`session/`) | **完了** |
| テスト | リンクパターン + テンプレート展開 + リネーム | sdit-core, sdit | **完了** |

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

## セキュリティレビュー結果

| 重要度 | ID | 概要 | 対応 |
|---|---|---|---|
| Medium | M-1 | file:/vbscript: 等の危険スキームがスキーム検証を通過 | **修正済み** — is_dangerous_scheme() にブロックリスト拡張 |
| Medium | M-2 | dfa_size_limit 未設定で実行時 ReDoS リスク | **修正済み** — dfa_size_limit(1 << 20) 追加 |
| Low | L-1 | セッション名の制御文字フィルタリング未実施 | 実害は低い |
| Low | L-2 | clamped_links がバイト長比較 | 厳しい側に倒れるため許容 |
| Low | L-3 | expand_template で巨大インデックスのフォールバック | captures.get で None 返却、安全 |
| Info | I-1 | ダブルクリック判定がウィンドウ横断で共有 | マルチウィンドウでの軽微な誤動作 |

## 依存関係

- Phase 8.2（URL 検出）— リンク検出基盤
