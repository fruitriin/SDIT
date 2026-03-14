#!/bin/bash
# Claude Code の settings.json パーミッションを
# Copilot CLI / Codex CLI 用フラグに変換するスクリプト
#
# Usage:
#   ./tools/permissions-to-flags.sh copilot   # Copilot CLI 用フラグ出力
#   ./tools/permissions-to-flags.sh codex     # Codex CLI 用フラグ出力

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGET="${1:-copilot}"

if [ "$TARGET" != "copilot" ] && [ "$TARGET" != "codex" ]; then
  echo "Usage: $0 [copilot|codex]" >&2
  exit 1
fi

PROJECT="$REPO_ROOT/.claude/settings.json"
LOCAL="$REPO_ROOT/.claude/settings.local.json"

# プロジェクト設定のみ使用（グローバルは含めない）
files=()
for f in "$PROJECT" "$LOCAL"; do
  [ -f "$f" ] && files+=("$f")
done

# 基本コマンド（読み取り系・ファイル探索系）をハードコードで補完
BASELINE='{"permissions":{"allow":[
  "Bash(git status:*)", "Bash(git log:*)", "Bash(git diff:*)",
  "Bash(git show:*)", "Bash(git blame:*)", "Bash(git branch:*)",
  "Bash(git fetch:*)", "Bash(git rev-parse:*)", "Bash(git stash list:*)",
  "Bash(ls:*)", "Bash(cat:*)", "Bash(grep:*)", "Bash(find:*)",
  "Bash(tree:*)", "Bash(wc:*)", "Bash(diff:*)", "Bash(tail:*)",
  "Bash(echo:*)", "Bash(which:*)", "Bash(test:*)", "Bash(ps:*)",
  "Bash(timeout:*)"
]}}'

# jq で全処理: マージ → 変換 → 重複排除 → フラグ出力
(echo "$BASELINE"; cat "${files[@]}") | jq -r --arg target "$TARGET" -s '
  # allow/deny をマージ
  [.[].permissions.allow // [] | .[]] as $allows |
  [.[].permissions.deny  // [] | .[]] as $denies |

  # 変換関数: Claude Code ルール → tool フラグ
  def convert:
    if test("^Bash\\(") then
      # Bash(cmd:*) → shell(cmd:*)
      sub("^Bash\\("; "shell(")
    elif . == "Edit" or . == "Write" or test("^Edit\\(") or test("^Write\\(") then
      "write"
    elif test("^(Read|WebFetch|WebSearch|Search|Skill)") then
      empty
    else
      empty
    end;

  # 変換・重複排除
  ([$allows[] | convert] | unique) as $allow_flags |
  ([$denies[] | convert] | unique) as $deny_flags |

  # git push を安全デフォルトで deny に追加
  ($deny_flags + (if ($deny_flags | index("shell(git push:*)")) then [] else ["shell(git push:*)"] end) | unique) as $deny_flags |

  # 出力
  (if $target == "copilot" then
    "# Copilot CLI flags\n# Usage: copilot -p \"prompt\" \\"
  else
    "# Codex CLI flags\n# Usage: codex exec --full-auto \\"
  end),
  ($allow_flags[] | "  --allow-tool=\u0027" + . + "\u0027 \\"),
  ($deny_flags[]  | "  --deny-tool=\u0027"  + . + "\u0027 \\"),
  "  \"your prompt here\""
'
