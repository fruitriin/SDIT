# Phase 14.2: スクロールバック設定

**概要**: スクロールバック履歴の最大行数を設定可能にする。現在 10,000 行にハードコードされている値を TOML で変更できるようにする。

**状態**: **完了**

## 背景

- `app.rs` の `SessionParams` で `scrollback: 10_000` がハードコード
- `Grid::new()` に `max_scroll_limit` として渡される
- ユーザーが大量のログ出力を遡りたい場合に制限が厳しい
- 逆に、メモリ節約のために小さい値にしたいユーザーもいる

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| ScrollbackConfig 追加 | `scrollback.lines` (u32, default: 10000, clamp: 0-1000000) | sdit-core (`config/mod.rs`) | **完了** |
| SessionParams 統合 | Config から scrollback 値を渡す | sdit (`app.rs`) | **完了** |
| Hot Reload 対応 | 設定変更時に新規セッションに反映（既存セッションは変更不可） | sdit (`event_loop.rs`) | **完了**（`self.config` 更新で自然に対応） |
| テスト | ScrollbackConfig serde 2件 + clamp 2件 + empty→default 1件 | sdit-core | **完了** |

## 設定例

```toml
[scrollback]
lines = 50000    # default: 10000, range: 0-1000000
```

## 依存関係

なし

## 実装メモ

- `ScrollbackConfig` を `config/mod.rs` に追加（`lines: u32`, default: 10_000, clamp: 0–1,000,000）
- `clamped_lines()` は `(lines as usize).min(1_000_000)` で実装
- `Config` に `pub scrollback: ScrollbackConfig` フィールドを追加
- `save_with_comments` に `[scrollback]` セクションのコメントを追加
- `app.rs` の `scrollback: 10_000` を `self.config.scrollback.clamped_lines()` に変更
- Hot Reload は既存の `self.config` 更新フローで自然に対応される（新規セッション作成時のみ反映）
