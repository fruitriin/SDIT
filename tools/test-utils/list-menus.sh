#!/bin/bash
# list-menus.sh
#
# osascript を使って macOS ネイティブメニューバーの構造を JSON で出力する。
# メニューバーが正しく設定されているかの確認に使用。
#
# Usage: ./list-menus.sh <process-name>
# Example: ./list-menus.sh sdit
#
# 出力例:
#   {"menus": [{"name": "SDIT", "items": ["About SDIT", "Preferences…", "Quit SDIT"]}, ...]}
#
# Exit codes:
#   0 — 成功
#   1 — 引数不正
#   2 — プロセスが見つからない、または権限エラー

set -euo pipefail

if [[ $# -ne 1 ]]; then
    echo "Usage: $0 <process-name>" >&2
    exit 1
fi

PROCESS_NAME="$1"

# プロセス名バリデーション
if [[ ! "$PROCESS_NAME" =~ ^[a-zA-Z0-9._-]+$ ]]; then
    echo "Error: process name must match [a-zA-Z0-9._-]+" >&2
    exit 1
fi

# PID ベースでプロセスを特定
PID=$(pgrep -x "$PROCESS_NAME" | head -1)
if [[ -z "$PID" ]]; then
    echo "Error: process '$PROCESS_NAME' not found" >&2
    exit 2
fi

osascript <<APPLESCRIPT
use framework "Foundation"
use scripting additions

tell application "System Events"
    set targetApp to first process whose unix id is $PID
    set menuBarItems to menu bar items of menu bar 1 of targetApp

    set jsonParts to {}
    repeat with menuBarItem in menuBarItems
        set menuName to name of menuBarItem
        try
            set menuItems to menu items of menu 1 of menuBarItem
            set itemNames to {}
            repeat with menuItem in menuItems
                set itemName to name of menuItem
                if itemName is not missing value then
                    set end of itemNames to "\"" & itemName & "\""
                end if
            end repeat
            set itemList to my joinList(itemNames, ", ")
            set end of jsonParts to "{\"name\": \"" & menuName & "\", \"items\": [" & itemList & "]}"
        on error
            set end of jsonParts to "{\"name\": \"" & menuName & "\", \"items\": []}"
        end try
    end repeat

    set jsonResult to my joinList(jsonParts, ", ")
    return "{\"menus\": [" & jsonResult & "]}"
end tell

on joinList(theList, delimiter)
    set oldDelims to AppleScript's text item delimiters
    set AppleScript's text item delimiters to delimiter
    set result to theList as text
    set AppleScript's text item delimiters to oldDelims
    return result
end joinList
APPLESCRIPT

STATUS=$?
if [[ $STATUS -ne 0 ]]; then
    echo "Error: osascript failed (exit $STATUS). Accessibility 権限を確認してください。" >&2
    exit 2
fi
