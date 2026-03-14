# Phase 21.2: シングルクリック時のキャレットマーカー表示を修正

## 問題

マウスクリックするとクリック位置にキャレットマーカー（1セル分の選択ハイライト）が表示される。ドラッグ選択時以外はマウス位置のキャレットマーカーは不要。

## 原因

`event_loop.rs` のシングルクリック処理で `Selection::new(Simple, point)` が作成され、`start == end` の1セル選択が残る。マウスリリース時に `is_selecting = false` にするだけで `selection` 自体はクリアされない。

## 修正方針

マウスリリース時に、ドラッグが発生しなかった場合（`selection.start == selection.end`）は `selection` を `None` にクリアし、再描画する。

## 変更対象

- `crates/sdit/src/event_loop.rs` — マウスリリース処理（行 1016 付近）

## 状態: 完了

## 実装内容

- `event_loop.rs` のマウスリリース処理で `sel.start == sel.end` なら `selection = None` にクリア

## セキュリティレビュー結果

問題なし
