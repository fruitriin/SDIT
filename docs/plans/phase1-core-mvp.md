# Phase 1 — sdit-core MVP

## 目的
ヘッドレスで動作するターミナルコアを実装する。
GUIなしでPTY起動・VTE処理・グリッド管理ができる状態をゴールとする。

## 前提条件
- Phase 0 のリファレンス読解が完了していること（最低限 Alacritty grid/pty/ansi）

## タスク
- [x] PTY起動・読み書き（Alacritty `tty/` 参照）— `crates/sdit-core/src/pty/mod.rs`
- [x] VTEパーサー統合（`vte` クレート使用、Alacritty `ansi.rs` 参照）— `crates/sdit-core/src/terminal/{mod,handler}.rs`
- [x] グリッドデータ構造（Alacritty `grid/` 参照）— `index.rs`, `grid/{cell,row,storage,mod}.rs`
- [x] ヘッドレステスト通過 — 63テスト（ユニット60 + 統合3）全通過

## 完了: 2026-03-11

## セキュリティレビュー結果

レビュー実施: 2026-03-11（計画レビュー）

### 修正済み
| 深刻度 | 問題 | 修正内容 |
|---|---|---|
| High | `scroll_up/down` で `region.end <= region.start` 時の usize アンダーフロー | 早期リターンガード追加 |
| High | `Grid::new(0,...)` → `Storage` ゼロ除算 panic | `lines.max(1)` でクランプ |
| Medium | CUU/CUD の `i32` 演算オーバーフロー（debug panic） | `saturating_sub/add` に変更 |
| Medium | ED/IL/DL の `line.0 as usize` で負値ラップアラウンド | `.max(0)` ガード追加 |
| Medium | `shrink_visible` の `debug_assert` が release で無効 | `min()` によるクランプに変更 |

### Low（Phase 1 内で修正済み）
| 深刻度 | 問題 | 修正内容 |
|---|---|---|
| Low | OSC タイトルの無制限文字列格納（メモリ消費 DoS） | `osc_dispatch` に `MAX_TITLE_BYTES = 4096` 上限を追加 |
| Low | `Line(i32)` の型安全性不足 | `Line::as_viewport_idx()` メソッド導入。散在する `.0.max(0) as usize` を統一 |

## 対象クレート
- `crates/sdit-core/`

## 参照
- `refs/alacritty/alacritty-terminal/src/tty/`
- `refs/alacritty/alacritty-terminal/src/ansi.rs`
- `refs/alacritty/alacritty-terminal/src/grid/`
