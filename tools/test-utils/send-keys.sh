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

# AppleScript インジェクション防止: バックスラッシュと二重引用符をエスケープ
ESCAPED_PROCESS="${PROCESS_NAME//\\/\\\\}"
ESCAPED_PROCESS="${ESCAPED_PROCESS//\"/\\\"}"
ESCAPED_TEXT="${TEXT//\\/\\\\}"
ESCAPED_TEXT="${ESCAPED_TEXT//\"/\\\"}"

# プロセスが存在するか確認
if ! pgrep -x "$PROCESS_NAME" > /dev/null 2>&1; then
    echo "Error: process '$PROCESS_NAME' not found" >&2
    exit 2
fi

# osascript でキーストローク送信
# System Events の keystroke は Unicode 文字列を受け付ける
osascript <<APPLESCRIPT
tell application "System Events"
    set targetApp to first process whose name is "$ESCAPED_PROCESS"
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

echo "Sent keystrokes to '$PROCESS_NAME': $TEXT"
