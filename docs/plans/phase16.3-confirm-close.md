# Phase 16.3: 閉じる前の確認ダイアログ

**概要**: セッションを閉じるとき（プロセスが実行中の場合）に確認ダイアログを表示する。

**状態**: **完了**

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| Config に confirm_close 設定追加 | `never`/`always`/`process_running`（デフォルト） | sdit-core (`config/mod.rs`) | **完了** |
| プロセス実行中判定 | シェルプロセスの子プロセスの有無を確認 | sdit-core (`session/session.rs`) | **完了** |
| 確認ダイアログ表示 | ターミナル内オーバーレイ "Close? [y/n]" | sdit (`confirm_close.rs`, `render.rs`) | **完了** |
| CloseSession/Quit アクションの変更 | 確認が必要な場合はダイアログ表示→確認後に実行 | sdit (`confirm_close.rs`, `event_loop.rs`) | **完了** |
| テスト | 設定デシリアライズ | sdit-core | **完了** |

## セキュリティレビュー結果

| 重要度 | ID | 概要 | 対応 |
|---|---|---|---|
| Low | L-1 | pgrep による子プロセス判定は安全 | 許容 |
| Low | L-2 | secure_input.rs の unsafe SAFETY コメント精度 | 実害なし |
| Low | L-3 | scrollbar の f64→usize キャスト | 実用範囲で安全 |
| Info | I-1 | minimum_contrast の NaN 入力 | clamped で安全 |
| Info | I-2 | confirm close バイパス不可 | 仕様通り |
| Info | I-3 | parse_hex_color の重複 | リファクタリング候補 |

## 設定例

```toml
[window]
confirm_close = "process_running"  # never | always | process_running
```

## 参照

- `refs/ghostty/src/config/Config.zig` — confirm-close-surface
- `refs/wezterm/wezterm-gui/src/overlay/confirm_close_pane.rs`
