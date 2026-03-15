# PTY 出力のフレームスロットリング

## 発見日
2026-03-15

## 問題
`ls -la` や `cat` などの大量出力コマンドで描画が極端に遅くなる。

## 根本原因
`SditEvent::PtyOutput` イベントが発火するたびに `redraw_session` を即座に呼んでいた。
`redraw_session` は全グリッドセルを CPU で処理し、GPU バッファに書き込み、`request_redraw` を発行する。
PTY から 1 回の read で得られるチャンク単位でイベントが来るため、大量出力時に数百〜数千回の再描画が発生していた。

## 解決策
`PtyOutput` では `dirty_sessions: HashSet<SessionId>` にフラグを立てるだけにし、
winit の `about_to_wait` コールバック（全イベント処理完了後に呼ばれる）で一括再描画する。

```rust
// PtyOutput ハンドラ
self.dirty_sessions.insert(session_id);

// about_to_wait
if !self.dirty_sessions.is_empty() {
    let frame_interval = self.config.window.max_fps.frame_interval();
    if self.last_pty_render.elapsed() >= frame_interval {
        let dirty: Vec<_> = self.dirty_sessions.drain().collect();
        for session_id in dirty {
            self.redraw_session(session_id);
        }
        self.last_pty_render = Instant::now();
    }
}
```

## 効果
- PTY 出力が何百回来ても、1 フレームにつき最大 1 回の再描画
- `max_fps` 設定で再描画頻度を制御可能（default=60fps, high=144fps, 数値指定）

## 設計メモ
- `about_to_wait` は winit のイベントループが全イベントを処理し終えた後に呼ばれる
- マウスドラッグ中の再描画は `redraw_session` を直接呼ぶ（即時フィードバックが必要なため）
- カーソル点滅も `about_to_wait` で処理しており、同じタイミングで統合されている
