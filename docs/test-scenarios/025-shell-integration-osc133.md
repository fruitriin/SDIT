# 025: シェルインテグレーション OSC 133 確認

**Phase**: 14.5
**状態**: UNIT_ONLY（ディスプレイスリープ中のため GUI テスト保留）

## 目的

OSC 133 A/B/C/D シーケンスによるセマンティックゾーンマーカーの記録と、
Cmd+Up/Down によるプロンプト間ジャンプが正しく動作することを確認する。

## テスト対象

- `SemanticZone` 型（PromptStart / CommandStart / OutputStart / CommandEnd）
- `Terminal::prev_prompt()` / `next_prompt()` によるプロンプト検索
- `ShellIntegrationConfig.enabled` による有効/無効切替
- fish シェル（組み込み OSC 133 出力）との互換性

---

## シナリオ 1: ユニットテスト（自動）

```bash
cargo test -p sdit-core -- terminal::tests::osc133
cargo test -p sdit-core -- terminal::tests::prompt_jump
```

### 期待結果

- `osc133_*` テスト 4件 PASS（A/B/C/D 各シーケンスのパース）
- `prompt_jump_*` テスト 2件以上 PASS（prev/next の境界条件含む）

---

## シナリオ 2: 設定有効/無効切替（ユニットテスト）

```bash
cargo test -p sdit-core -- config::tests::shell_integration
```

`ShellIntegrationConfig::default().enabled == true` を確認。

---

## シナリオ 3: GUI テスト — fish シェルでプロンプトジャンプ（Display 起動時に実施）

### 前提条件

- SDIT バイナリが起動可能であること
- fish がインストールされていること（`which fish`）
- ディスプレイが起動中であること（`Display Asleep: No`）

### 手順

1. SDIT 起動: `./target/debug/sdit`
2. fish を起動: `fish` → Enter
3. コマンドをいくつか実行（プロンプトを複数生成する）:
   ```
   echo hello
   ls -la
   pwd
   ```
4. Cmd+Up キーを押す → 前のプロンプト行へスクロールすることを確認
5. Cmd+Up を続けて押す → さらに前のプロンプトへ移動することを確認
6. Cmd+Down キーを押す → 次のプロンプト行へ戻ることを確認
7. スクリーンショット撮影: `capture-window --pid <SDIT_PID> tmp/025-prompt-jump.png`

### 期待結果

- プロンプト行の直前の行が画面上部に表示される
- 複数回の Up/Down でプロンプト間を順番に移動できる

---

## シナリオ 4: GUI テスト — ShellIntegration 無効時のフォールバック（Display 起動時に実施）

### 手順

1. `~/.config/sdit/config.toml` に以下を追加:
   ```toml
   [shell_integration]
   enabled = false
   ```
2. SDIT 起動
3. fish 起動 → コマンド実行
4. Cmd+Up → スクロールしないこと（プロンプトジャンプが無効）を確認

### 期待結果

- Cmd+Up は通常スクロール（または無操作）として扱われる
- プロンプトジャンプは発生しない

---

## ユニットテストのみで確認できる検証項目

| 項目 | テスト | 状態 |
|---|---|---|
| OSC 133;A → PromptStart | `osc_133_prompt_start` | PASS |
| OSC 133;B → CommandStart | `osc_133_command_start` | PASS |
| OSC 133;C → OutputStart | `osc_133_output_start` | PASS |
| OSC 133;D;0 → CommandEnd(0) | `osc_133_command_end_with_exit_code` | PASS |
| OSC 133;D (コード無し) → CommandEnd(None) | `osc_133_command_end_no_exit_code` | PASS |
| shell_integration 無効時は記録しない | `osc_133_disabled_when_shell_integration_off` | PASS |
| prev_prompt / next_prompt ナビゲーション | `prompt_navigation` | PASS |
| MAX_SEMANTIC_MARKERS 超過時の古いマーカー破棄 | `semantic_markers_capped` | PASS |

---

## 注記

- fish は組み込みで OSC 133 A/B/C を出力するため、追加設定不要
- bash/zsh は `precmd`/`PS1` フック設定が別途必要（本シナリオの対象外）
- display_offset の計算: `new_offset = (history_size - target_line).clamp(0, history_size)`
