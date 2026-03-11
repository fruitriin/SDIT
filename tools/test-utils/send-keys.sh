#!/bin/bash
# send-keys.sh
#
# osascript (System Events) を使って指定プロセスにキーストロークを送信する。
#
# Usage: ./send-keys.sh <process-name> <text>
# Example: ./send-keys.sh sdit "hello world"
#
# 前提条件:
#   - System Settings → Privacy & Security → Accessibility で
#     このスクリプトを実行するターミナルに権限が必要
#
# Exit codes:
#   0 — 成功
#   1 — 引数不正
#   2 — プロセスが見つからない、または権限エラー

set -euo pipefail

if [[ $# -ne 2 ]]; then
    echo "Usage: $0 <process-name> <text>" >&2
    exit 1
fi

PROCESS_NAME="$1"
TEXT="$2"

# プロセス名バリデーション（正規表現メタキャラクタ防止: M-NEW-1 対応）
if [[ ! "$PROCESS_NAME" =~ ^[a-zA-Z0-9._-]+$ ]]; then
    echo "Error: process name must match [a-zA-Z0-9._-]+" >&2
    exit 1
fi

# AppleScript インジェクション防止: バックスラッシュと二重引用符をエスケープ
ESCAPED_TEXT="${TEXT//\\/\\\\}"
ESCAPED_TEXT="${ESCAPED_TEXT//\"/\\\"}"

# M-3: PID ベースでプロセスを特定（basename なりすまし防止）
PID=$(pgrep -x "$PROCESS_NAME" | head -1)
if [[ -z "$PID" ]]; then
    echo "Error: process '$PROCESS_NAME' not found" >&2
    exit 2
fi

# osascript でキーストローク送信（PID ベースのプロセス指定）
osascript <<APPLESCRIPT
tell application "System Events"
    set targetApp to first process whose unix id is $PID
    set frontmost of targetApp to true
    delay 0.1
    keystroke "$ESCAPED_TEXT"
end tell
APPLESCRIPT

STATUS=$?
if [[ $STATUS -ne 0 ]]; then
    echo "Error: osascript failed (exit $STATUS). Accessibility 権限を確認してください。" >&2
    exit 2
fi

echo "Sent keystrokes to '$PROCESS_NAME' (pid=$PID): $TEXT"
