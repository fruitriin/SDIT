#!/bin/bash
# click-menu.sh
#
# osascript を使って macOS ネイティブメニューバーのメニュー項目をクリックする。
#
# Usage: ./click-menu.sh <process-name> <menu-name> [menu-item]
# Example:
#   ./click-menu.sh sdit "File"              # File メニューを開く
#   ./click-menu.sh sdit "File" "New Window" # File > New Window をクリック
#
# Exit codes:
#   0 — 成功
#   1 — 引数不正
#   2 — プロセスが見つからない、メニューが見つからない、または権限エラー

set -euo pipefail

if [[ $# -lt 2 || $# -gt 3 ]]; then
    echo "Usage: $0 <process-name> <menu-name> [menu-item]" >&2
    exit 1
fi

PROCESS_NAME="$1"
MENU_NAME="$2"
MENU_ITEM="${3:-}"

# プロセス名バリデーション
if [[ ! "$PROCESS_NAME" =~ ^[a-zA-Z0-9._-]+$ ]]; then
    echo "Error: process name must match [a-zA-Z0-9._-]+" >&2
    exit 1
fi

# AppleScript インジェクション防止
ESCAPED_MENU="${MENU_NAME//\\/\\\\}"
ESCAPED_MENU="${ESCAPED_MENU//\"/\\\"}"
ESCAPED_ITEM="${MENU_ITEM//\\/\\\\}"
ESCAPED_ITEM="${ESCAPED_ITEM//\"/\\\"}"

# PID ベースでプロセスを特定
PID=$(pgrep -x "$PROCESS_NAME" | head -1)
if [[ -z "$PID" ]]; then
    echo "Error: process '$PROCESS_NAME' not found" >&2
    exit 2
fi

if [[ -z "$MENU_ITEM" ]]; then
    # メニューを開くだけ
    osascript <<APPLESCRIPT
tell application "System Events"
    set targetApp to first process whose unix id is $PID
    set frontmost of targetApp to true
    delay 0.2
    tell targetApp
        click menu bar item "$ESCAPED_MENU" of menu bar 1
    end tell
end tell
APPLESCRIPT
    STATUS=$?
else
    # メニュー項目をクリック
    osascript <<APPLESCRIPT
tell application "System Events"
    set targetApp to first process whose unix id is $PID
    set frontmost of targetApp to true
    delay 0.2
    tell targetApp
        click menu item "$ESCAPED_ITEM" of menu "$ESCAPED_MENU" of menu bar item "$ESCAPED_MENU" of menu bar 1
    end tell
end tell
APPLESCRIPT
    STATUS=$?
fi

if [[ $STATUS -ne 0 ]]; then
    echo "Error: osascript failed (exit $STATUS). メニュー名/項目名を確認してください。" >&2
    exit 2
fi

if [[ -z "$MENU_ITEM" ]]; then
    echo "Opened menu '$MENU_NAME' in '$PROCESS_NAME' (pid=$PID)"
else
    echo "Clicked '$MENU_NAME' > '$MENU_ITEM' in '$PROCESS_NAME' (pid=$PID)"
fi
