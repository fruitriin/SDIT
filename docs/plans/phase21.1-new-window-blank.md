# Phase 21.1: 新ウィンドウ空白表示の修正

## 問題

Cmd+N で新ウィンドウを開くと、シェルが起動してプロンプトを出力するまでの間、カーソル（キャレット）しか表示されず「何を操作しているのかわからない」状態になる。

## 原因

`window_ops.rs` の `create_window_with_cwd` で、PTY セッション生成直後に `redraw_session` を呼ぶが、シェルがまだ起動していないためグリッドが完全に空。カーソル（row=0, col=0）だけが描画される。

## 修正方針

1. **背景色の即時描画**: シェル出力が来るまでの間、空グリッドでもウィンドウ全体に背景色を塗りつぶして描画する（現状は空セルが透明扱いになっている可能性を確認）
2. **新ウィンドウのフォーカス**: `create_window` 後に `window.focus_window()` を呼び、新ウィンドウにフォーカスを移す（winit 0.30 の API を確認）

## 変更対象

- `crates/sdit/src/window_ops.rs` — 新ウィンドウ生成後のフォーカス処理
- `crates/sdit/src/render.rs` — 空グリッド時の背景描画確認

## 状態: 完了

## 実装内容

- `window_ops.rs` の `create_window_with_cwd` で `self.windows.insert(...)` 後に `focus_window()` を呼び出し

## セキュリティレビュー結果

問題なし
