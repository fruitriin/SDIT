# Phase 14.5: シェルインテグレーション（OSC 133）

**概要**: OSC 133 シーケンスでシェルのプロンプト境界を検出し、プロンプト間ジャンプやコマンド出力の選択を可能にする。

**状態**: **完了**

## 実装結果

- `SemanticZone` enum + `SemanticMarker` struct を `terminal/mod.rs` に追加
- `osc_dispatch` で OSC 133 A/B/C/D を解釈（`terminal/mod.rs`）
- `VecDeque<SemanticMarker>` で時系列順にマーカーを記録（上限 10,000）
- `prev_prompt()` / `next_prompt()` メソッドでプロンプト間ナビゲーション
- `ShellIntegrationConfig { enabled: bool }` を `config/mod.rs` に追加
- `Action::PrevPrompt` / `Action::NextPrompt` キーバインド（Cmd+Up/Down）
- テスト 7件追加（パース4件 + ナビゲーション1件 + キャップ1件 + 無効化1件）

### セキュリティレビュー結果

| 重要度 | ID | 内容 | 対応 |
|---|---|---|---|
| Medium | M-1 | `Vec::remove(0)` は O(n) → DoS リスク | `VecDeque` + `pop_front()` に修正済み |
| Low | L-1 | exit_code が未バリデーション | `clamp(0, 255)` 追加済み |
| Low | L-2 | マーカーの line フィールドがカーソル行のみ | Plan に記録（将来改善） |

## 背景

- OSC 133 はシェル（bash/zsh/fish）がプロンプト・コマンド・出力の境界をターミナルに通知する仕組み
- これにより「前のプロンプトにジャンプ」「コマンド出力を選択」等の高度な操作が可能になる
- Ghostty, WezTerm, iTerm2 等が対応済み
- fish は組み込みで OSC 133 を出力する。bash/zsh は設定が必要

## OSC 133 シーケンス

| シーケンス | 意味 |
|---|---|
| `ESC ] 133 ; A ST` | プロンプト開始 |
| `ESC ] 133 ; B ST` | コマンド入力開始（プロンプト終了） |
| `ESC ] 133 ; C ST` | コマンド出力開始 |
| `ESC ] 133 ; D ; <exit_code> ST` | コマンド終了 + 終了コード |

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| SemanticZone 型 | PromptStart/CommandStart/OutputStart/CommandEnd(exit_code) | sdit-core (`terminal/mod.rs`) | **完了** |
| OSC 133 パーサー | osc_dispatch で A/B/C/D を解釈 | sdit-core (`terminal/mod.rs`) | **完了** |
| ゾーンマーカー記録 | VecDeque でマーカー記録（上限10K） | sdit-core (`terminal/mod.rs`) | **完了** |
| プロンプトジャンプ | Cmd+Up/Down でプロンプト間を移動 | sdit (`event_loop.rs`, `config/keybinds.rs`) | **完了** |
| テスト | OSC 133 パース 4件 + ナビゲーション + キャップ + 無効化 | sdit-core | **完了** |

## 設定例

```toml
[shell_integration]
enabled = true    # default: true
```

## 参照

- `refs/ghostty/src/termio/shell_integration.zig` — シェルインテグレーション実装
- FinalTerm spec: OSC 133 の仕様元

## 依存関係

なし
