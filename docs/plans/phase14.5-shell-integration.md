# Phase 14.5: シェルインテグレーション（OSC 133）

**概要**: OSC 133 シーケンスでシェルのプロンプト境界を検出し、プロンプト間ジャンプやコマンド出力の選択を可能にする。

**状態**: 未着手

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
| SemanticZone 型 | PromptStart/CommandStart/OutputStart/CommandEnd(exit_code) | sdit-core (`terminal/mod.rs`) | 未着手 |
| OSC 133 パーサー | osc_dispatch で A/B/C/D を解釈 | sdit-core (`terminal/handler.rs`) | 未着手 |
| ゾーンマーカー記録 | Grid の各行にゾーン情報を保持 | sdit-core (`grid/mod.rs`) | 未着手 |
| プロンプトジャンプ | Cmd+Up/Down でプロンプト間を移動 | sdit (`input.rs`, `event_loop.rs`) | 未着手 |
| テスト | OSC 133 パース 4件 + ゾーン記録 2件 | sdit-core | 未着手 |

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
