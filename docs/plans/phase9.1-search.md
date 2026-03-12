# Phase 9.1: スクロールバック内検索

**概要**: Cmd+F でスクロールバックバッファ内のテキスト検索を行う機能を実装する。

**状態**: **完了**

## タスク

| タスク | 詳細 | 変更先クレート | 状態 |
|---|---|---|---|
| 検索エンジン | Grid 全行走査 + case-insensitive 部分一致検索 | sdit-core (`terminal/search.rs`) | 完了 |
| 検索ショートカット | Cmd+F, Escape, Enter/Shift+Enter, Cmd+G/Cmd+Shift+G | sdit (`input.rs`) | 完了 |
| SearchState | 検索状態管理（query, matches, current_match） | sdit (`app.rs`) | 完了 |
| マッチハイライト | Yellow(#f9e2af) / Peach(#fab387) + HashSet O(1)ルックアップ | sdit-core (`render/pipeline.rs`) | 完了 |
| 検索バーUI | 最終行オーバーレイ `> {query} [{n}/{m}]` | sdit (`render.rs`) | 完了 |
| イベント統合 | 検索モードハンドラ + IME対応 + スクロール連動 | sdit (`event_loop.rs`) | 完了 |

## 依存関係

Phase 8

## セキュリティレビュー結果

### 修正済み (Medium)

- **M-1**: クエリ長制限（1000文字）— `event_loop.rs` の `push_str` 2箇所にガード追加
- **M-2**: マッチ数上限（10,000件）— `search.rs` の `SearchEngine::search` に早期リターン追加
- **M-3**: パイプラインのマッチルックアップ最適化 — `pipeline.rs` で `O(N×M)` の線形探索を `HashSet<(usize, usize)>` による `O(1)` に改善

### 記録のみ (Low / Info)

- **L-1**: `to_lowercase()` による Unicode 正規化の不完全性（ß→ss 等でバイト長が変わる）— 現実的な影響は軽微、将来の Unicode 正規化対応で解消
- **L-2**: `byte_offset_to_col` の `chars().count()` が `O(N)` — 現在のクエリ長制限（1000文字）で十分高速
- **L-3**: 検索結果の `Vec<SearchMatch>` がメモリを保持し続ける — マッチ数上限（10,000件）により問題なし
- **L-4**: `display_offset_for_match` の off-by-one リスク — テストで検証済み、edge case は軽微
- **I-1**: 検索バー overwrite_cell がセル数を超えるクエリで切り詰め — 意図的な設計
- **I-2**: IME プリエディットと検索モードの相互作用 — 現在は検索中の IME Commit のみ対応、Preedit は無視で問題なし
