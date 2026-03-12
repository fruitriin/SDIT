# Phase 8.1: フォントサイズ動的変更

**概要**: Cmd+=/- でフォントサイズをリアルタイム変更できるようにする。

**状態**: **完了**

## タスク

| タスク | 詳細 | 変更先クレート | 状態 |
|---|---|---|---|
| Cmd+=/- でフォントサイズ変更 | `FontContext` のサイズ変更 + メトリクス再計算 | sdit-core (`font.rs`) | 完了 |
| アトラスのクリアと再構築 | グリフキャッシュを全クリアし、新サイズで再ラスタライズ | sdit-core (`atlas.rs`) | 完了 |
| Terminal リサイズ連動 | フォントサイズ変更 → セルサイズ変化 → 全セッションリサイズ | sdit (`app.rs`) | 完了 |
| Cmd+0 でデフォルトサイズ復帰 | 設定ファイルのフォントサイズに復帰 | sdit (`app.rs`, `event_loop.rs`) | 完了 |

## 実装詳細

- `Atlas::clear()` — shelves/data/dirty をリセット
- `FontContext::set_font_size()` — font_size 変更 + metrics 再計算 + glyph_cache クリア（1.0〜200.0 にクランプ）
- `is_zoom_in/out/reset_shortcut()` — Cmd+=/Cmd+-/Cmd+0 判定
- `SditApp::change_font_size(Option<f32>)` — `Some(delta)` で差分変更、`None` でデフォルト復帰
- `event_loop.rs` にショートカットハンドラ統合

## セキュリティレビュー結果

| 重要度 | ID | 内容 | 対応 |
|---|---|---|---|
| Medium | M-1 | `delta == 0.0` の浮動小数点等値比較 | **修正済み**: `Option<f32>` に変更 |
| Low | L-1 | ズームステップが設定変更不可（ハードコード 1.0pt） | 将来の Config 拡張で対応 |
| Low | L-2 | キーリピートによるアトラス再構築の連続発火 | デバウンス処理を将来追加 |
| Low | L-3 | `try_into().unwrap_or` で PTY サイズ乖離の可能性 | 極端な条件のみ。将来クランプ追加 |
| Info | I-1 | `default_font_size` が全セッション共有 | 現仕様では問題なし |
| Info | I-2 | ズームショートカットが macOS 専用 | cross-platform 対応時に追加 |

## 依存関係

なし
