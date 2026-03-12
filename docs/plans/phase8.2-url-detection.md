# Phase 8.2: URL検出・クリック

**概要**: ターミナル出力内のURLを検出し、Cmd+クリックでブラウザを開く機能を実装する。

**状態**: **完了**

## タスク

| タスク | 詳細 | 変更先クレート | 状態 |
|---|---|---|---|
| OSC 8 ハイパーリンク対応 | Cell に `Option<Arc<str>>` で URL 保持 | sdit-core (`cell.rs`, `terminal/mod.rs`) | 完了 |
| URL正規表現検出 | `https?://...` パターン検出（regex クレート） | sdit-core (`terminal/url_detector.rs`) | 完了 |
| Cmd+クリックでブラウザ起動 | `open` コマンド実行（セキュリティ二重チェック付き） | sdit (`event_loop.rs`, `input.rs`) | 完了 |
| URL アンダーライン表示 | Cmd 押下中のマウスホバーで URL 青色強調 | sdit (`render.rs`), sdit-core (`pipeline.rs`) | 完了 |

## 実装詳細

- `Cell.hyperlink: Option<Arc<str>>` — OSC 8 ハイパーリンク URL を保持
- `Terminal.current_hyperlink` — アクティブなハイパーリンク状態
- `UrlDetector` — regex ベースのオンデマンド URL 検出（OSC 8 優先）
- `is_url_modifier()` — Cmd (macOS) / Ctrl (他) 判定
- `update_url_hover()` — Cmd+ホバーで URL セルを青色表示 + Pointer カーソル
- Cmd+Click → `open` / `xdg-open` でブラウザ起動

## セキュリティレビュー結果

| 重要度 | ID | 内容 | 対応 |
|---|---|---|---|
| Medium | M-1 | URL 内の制御文字バリデーション | **修正済み**: `bytes().all(\|b\| b >= 0x20 && b != 0x7F)` チェック追加 |
| Medium | M-2 | find_url_at のディフェンスインデプス | **修正済み**: OSC 8 URL にも http/https 再確認追加 |
| Low | L-1 | URL 検出の2回スキャン（パフォーマンス） | 行幅が制限されているため許容 |
| Low | L-2 | ホバー時の Cell クローン + String 割り当て | 実害なし。将来デバウンス検討 |
| Low | L-3 | Alt Screen 切替時の current_hyperlink リセット漏れ | **修正済み**: swap_alt_screen に None クリア追加 |
| Info | I-1 | Arc&lt;str&gt; のアロケーション重複 | 実用上問題なし |
| Info | I-2 | ReDoS リスクなし | regex クレートは DFA/NFA ハイブリッド |

## 依存関係

Phase 6.1（マウスイベント基盤）
