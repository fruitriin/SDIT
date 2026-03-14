# Codex CLI / Copilot CLI へのタスク移譲ノウハウ

Claude Code のトークン消費を抑えるため、実装タスクを Codex CLI や Copilot CLI に移譲する際の知見。

## ツール概要

| | Codex CLI | Copilot CLI |
|---|---|---|
| コマンド | `codex exec` | `copilot -p` |
| 認証 | ChatGPT Pro ($200/月) | GitHub Pro ($10/月) |
| 課金 | 300-1500 msg/5h (ChatGPT認証) | 1プロンプト=1 premium request。GPT-5 mini は消費なし |
| サンドボックス | あり（workspace-write） | なし（Docker推奨） |
| 指示ファイル | AGENTS.md（ネイティブ）、CLAUDE.md（fallback設定可） | AGENTS.md / CLAUDE.md 両方読む |

## 指示ファイルの互換性

`AGENTS.md` を実ファイルとして置き、`CLAUDE.md` と `CLAUDE.local.md` を読むよう指示する:

```markdown
# AGENTS.md
1. `CLAUDE.md` — プロジェクト全体の設計指針（コミット済み）
2. `CLAUDE.local.md` — ローカル固有の規約（存在する場合のみ読む）
```

- Claude Code: `CLAUDE.md` / `CLAUDE.local.md` をネイティブ読み
- Codex CLI: `AGENTS.md` をネイティブ読み → `CLAUDE.md` / `CLAUDE.local.md` を読む指示に従う
- Copilot CLI: `AGENTS.md` を primary instructions として読む → 同上

**`CLAUDE.local.md` は Copilot/Codex に直接読まれない**（`.local` バリアントは非対応）。
`AGENTS.md` に「`CLAUDE.local.md` を読め」と書くことで間接的に対応する。

Codex で `CLAUDE.md` を直接読む場合は `~/.codex/config.toml`:
```toml
project_doc_fallback_filenames = ["CLAUDE.md"]
```

## パーミッション変換

`tools/permissions-to-flags.sh` で Claude Code の settings.json を各ツール用フラグに変換:
```bash
./tools/permissions-to-flags.sh copilot   # Copilot 用
./tools/permissions-to-flags.sh codex     # Codex 用
```

## モデルの振る舞いのクセ

### GPT-5 Codex 系（Codex CLI デフォルト）

**得意:**
- バックエンドロジック、リファクタリング、デバッグ
- ターミナルベースのコーディング（Sonnet 4.6 比 +18pt）
- 長時間の自律作業（数時間連続で動ける設計）
- トークン効率が良い（Claude 比 2-4x 少ないトークンで同等タスク）

**苦手・クセ:**
- 要件の誤解が起きやすい → プロンプトで明示的に制約を書く
- コード膨張（同等タスクで 3x のコード量）→ 「最小限の変更で」と指示する
- UI/フロントエンド系は弱い
- 曖昧な仕様から意図を汲み取る力が Claude より弱い

**推奨 reasoning effort:**
- 対話的: `medium`（速度と品質のバランス）
- 自律実行: `high` or `xhigh`（難しいタスク向け）

### GPT-5 mini（Copilot CLI デフォルト・premium request 消費なし）

**得意:**
- 軽量タスク（テスト追加、フォーマット修正、単純なリファクタ）
- 高速レスポンス

**苦手・クセ:**
- 複雑な設計判断は精度が落ちる
- Rust の高度な型パズル（lifetime、trait bound）は苦戦する可能性

### Claude Sonnet 4.6（Claude Code サブエージェント）

**得意:**
- 曖昧・不完全な仕様からの意図理解
- CLAUDE.md/プロジェクト規約の遵守
- Rust コードの型安全な実装

**苦手:**
- トークン消費が大きい

## プロンプトのコツ

### 共通原則

1. **具体的に書く** — 「このファイルのこの関数を修正して」> 「バグを直して」
2. **検証手順を含める** — 「変更後 `cargo test` と `cargo clippy` を実行して確認して」
3. **最小限を指示する** — 「最小限の変更で」「関係ないコードは触らないで」
4. **期待する出力を示す** — 入力例と期待出力を書く

### Codex CLI 固有

```bash
# 良い例: 検証ループを含める
codex exec --full-auto \
  "crates/sdit-core/src/terminal/ansi.rs の OSC 52 ハンドラに
   base64 デコードのエラーハンドリングを追加して。
   変更後 cargo test -p sdit-core を実行し、パスすることを確認して。
   最小限の変更で。"

# reasoning effort を上げる（難しいタスク）
codex exec --full-auto --reasoning=high "..."
```

- `--full-auto` = sandbox(workspace-write) + 承認なし。安全かつ自律的
- sandbox がネットワークをブロックするので、`cargo` が依存を fetch する必要がある場合:
  ```bash
  codex exec --full-auto -c 'sandbox_workspace_write.network_access=true' "..."
  ```

### Copilot CLI 固有

```bash
# 良い例: plan mode で計画を立てさせてから実行
copilot -p "以下のタスクを plan mode で計画し、autopilot で実行して:
  crates/sdit/src/window.rs の close_window メソッドに
  確認ダイアログを追加する。
  変更後 cargo build && cargo test を実行。" \
  --allow-tool='write' \
  --allow-tool='shell(cargo:*)' \
  --allow-tool='shell(git:*)' \
  --deny-tool='shell(git push:*)'

# クラウド委譲（ターミナルを解放したい場合）
# 対話モードで & プレフィックスを使う
copilot
> & このタスクをやって...
```

- `--allow-tool` で必要最小限の権限を渡す（`--allow-all-tools` は避ける）
- premium request を消費しない GPT-5 mini で十分なタスク: テスト追加、fmt/clippy 修正、ドキュメント更新
- 複雑なタスクは `-m claude-sonnet-4-6` でモデル指定も可能（premium request 消費）

## タスクの振り分け基準

| タスク種別 | 推奨ツール | 理由 |
|---|---|---|
| 単純な実装（テスト追加、typo修正） | Copilot CLI (GPT-5 mini) | 無料、高速 |
| 中規模実装（機能追加、リファクタ） | Codex CLI (full-auto) | sandbox安全、自律的 |
| SDIT 固有規約が重要なタスク | Claude サブエージェント | CLAUDE.md 深い理解 |
| 設計判断・曖昧な仕様 | Claude（メイン or サブ） | 意図理解力 |
| セキュリティレビュー | Claude（メイン） | 信頼性重視 |

## 注意事項

- `.claude/hooks/` は Codex/Copilot には効かない（AGENTS.md の指示文のみで制約）
- worktree 統合は手動（`git worktree add` → `cd` → 実行）
- Codex の sandbox は `/tmp/` への書き込みもブロックする → プロジェクトの `tmp/` 方針と自然に整合
- Copilot の `--allow-all-tools` はホスト直接実行で危険 → 個別 `--allow-tool` を使う

### `cargo test` はGUIテストでハングする

`cargo test --workspace` は `smoke_gui` / `gui_interaction` を含むため、
Copilot の非TTY環境では SDIT バイナリ起動待ちでハング（120秒以上）する。

プロンプトには必ず以下を指示すること:

```bash
# ✅ 正しい検証コマンド
cargo test -p sdit-core && cargo test -p sdit --lib

# ❌ ハングする
cargo test
cargo test --workspace
```

## 参考リンク

- [Codex Prompting Guide](https://developers.openai.com/cookbook/examples/gpt-5/codex_prompting_guide/)
- [Codex Best Practices](https://developers.openai.com/codex/learn/best-practices/)
- [Copilot CLI Best Practices](https://docs.github.com/en/copilot/how-tos/copilot-cli/cli-best-practices)
- [Copilot CLI Task Delegation](https://docs.github.com/en/copilot/how-tos/copilot-cli/use-copilot-cli-agents/delegate-tasks-to-cca)
- [GPT-5 Codex vs Claude Sonnet 4.5](https://composio.dev/content/claude-sonnet-4-5-vs-gpt-5-codex-best-model-for-agentic-coding)
