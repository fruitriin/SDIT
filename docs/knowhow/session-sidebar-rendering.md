# サイドバー描画とセッション管理ノウハウ

## origin_x によるレイアウト分割

ターミナル描画を右にオフセットするため Uniforms に `origin_x` を追加。
WGSL シェーダで `screen += vec2(origin_x, 0.0)` するだけで位置調整が完了する。

- サイドバーパイプライン: `origin_x = 0`
- ターミナルパイプライン: `origin_x = sidebar_width_px`
- 同一レンダーパスで2回 draw するだけなのでオーバーヘッドは最小限

## CellPipeline 再利用によるサイドバー描画

サイドバーは CellVertex 配列を手動生成して `update_cells()` で書き込む。
既存の CellPipeline（インスタンス描画）をそのまま流用できる。

- `build_sidebar_cells()` で各行に対応する CellVertex を生成
- フォントラスタライズは `font_ctx.rasterize_glyph()` を直接呼び出し
- Catppuccin Mocha カラースキームでアクティブ行をハイライト

## セッション切出し時の安全なロールバック

`detach_session_to_new_window()` では元ウィンドウからセッションを先に除去し、
新ウィンドウ/GPU 作成に失敗した場合はロールバックする。

- 元のインデックス位置を保存し、`insert(original_index, sid)` で復元
- `push()` ではなく `insert()` を使うことで順序の一貫性を維持

## サイドバー内ドラッグのパターン

winit の `CursorMoved` + `MouseInput` で実装:

1. `MouseInput::Pressed` でドラッグ開始行を記録
2. `CursorMoved` で行が変わったら `sessions.swap()` + active_index 追従
3. `MouseInput::Released` でドラッグ状態をクリア

## サイドバー幅を考慮したグリッドサイズ計算

リサイズ時と新規セッション追加時、サイドバー幅を差し引いた幅でグリッド列数を計算:
```
let term_width = (surface_width - sidebar_width_px).max(0.0);
let (cols, rows) = calc_grid_size(term_width, height, cell_width, cell_height);
```

## ChildExit での削除順序

セッション削除は以下の順序で実行する（整合性のため統一）:
1. `ws.sessions.remove()` — UI 状態を先に更新
2. `session_to_window.remove()` — マッピングを除去
3. `session_mgr.remove()` — PTY の Drop（SIGHUP 送信）は最後
