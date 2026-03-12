# Phase 5.9: main.rs 分割リファクタリング

## 背景

`crates/sdit/src/main.rs` が1292行に達しており、Phase 6以降でマウス処理・キーバインド・
IME等を追加すると2000〜3000行に膨れる見込み。God Object化を防ぐため、
Phase 6（マウス）着手前に分割する。

## 現状の責務分析

main.rs に混在している責務:

1. **アプリ状態管理**: `App` 構造体、セッション管理、ウィンドウレジストリ
2. **イベントループ**: winit `EventLoop::run` のコールバック
3. **キー入力処理**: `key_to_bytes()`、ショートカット判定
4. **ウィンドウ操作**: 生成、破棄、リサイズ、フォーカス管理
5. **レンダリング呼び出し**: wgpu サーフェス取得、パイプライン呼び出し
6. **サイドバーUI**: サイドバー描画ロジックの統合

## 分割方針

```
crates/sdit/src/
├── main.rs          # エントリーポイント（EventLoop::run のみ）
├── app.rs           # App 構造体・アプリ全体の状態
├── input.rs         # キー入力 → PTY バイト列変換、ショートカット判定
├── window.rs        # ウィンドウ生成・破棄・リサイズ・フォーカス管理
└── event_loop.rs    # winit イベントハンドラの分岐ロジック
```

## タスク

| タスク | 詳細 | 工数 |
|---|---|---|
| App 構造体を `app.rs` に抽出 | フィールドとヘルパーメソッドを移動 | 小 |
| `key_to_bytes()` + ショートカット判定を `input.rs` に抽出 | Phase 6 でマウス処理、Phase 9 でキーバインドが入る場所 | 小 |
| ウィンドウ操作を `window.rs` に抽出 | `create_window()`, `close_window()` 等 | 小 |
| イベントハンドラを `event_loop.rs` に抽出 | `WindowEvent` のマッチ分岐を分離 | 中 |
| テスト通過確認 | `cargo test` + GUI スモークテスト | 小 |

## 完了条件

- main.rs が 100行以下（エントリーポイントのみ）
- 各ファイルが 400行以下
- 既存テスト全パス
- 機能変更なし（純粋なリファクタリング）

## 依存関係

- 前提: Phase 5.5（ターミナル互換性）完了後
- 後続: Phase 6（マウス）の前提。input.rs がマウス処理の受け入れ先になる

## リファレンス

- `refs/alacritty/alacritty/src/` — `event.rs`, `input/mod.rs`, `window/mod.rs` に分離されている
- `refs/ghostty/src/App.zig`, `src/Surface.zig` — App/Surface の明確な分離
