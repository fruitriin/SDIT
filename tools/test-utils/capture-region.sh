#!/bin/bash
# capture-region.sh
#
# macOS の screencapture -R を使って画面の指定領域を PNG キャプチャする。
# メニューバーやスクリーン座標ベースの UI テストに使用。
#
# Usage: ./capture-region.sh <x> <y> <width> <height> <output-path>
# Example: ./capture-region.sh 0 0 400 25 tmp/menubar.png
#
# Exit codes:
#   0 — 成功
#   1 — 引数不正
#   2 — screencapture 失敗

set -euo pipefail

if [[ $# -ne 5 ]]; then
    echo "Usage: $0 <x> <y> <width> <height> <output-path>" >&2
    exit 1
fi

X="$1"
Y="$2"
WIDTH="$3"
HEIGHT="$4"
OUTPUT_PATH="$5"

# 数値バリデーション
for val in "$X" "$Y" "$WIDTH" "$HEIGHT"; do
    if [[ ! "$val" =~ ^[0-9]+$ ]]; then
        echo "Error: coordinates must be non-negative integers" >&2
        exit 1
    fi
done

# 出力先ディレクトリを作成
mkdir -p "$(dirname "$OUTPUT_PATH")"

# screencapture -R で領域キャプチャ
/usr/sbin/screencapture -R"${X},${Y},${WIDTH},${HEIGHT}" "$OUTPUT_PATH"
STATUS=$?

if [[ $STATUS -ne 0 ]]; then
    echo "Error: screencapture failed (exit $STATUS)" >&2
    exit 2
fi

if [[ ! -f "$OUTPUT_PATH" ]]; then
    echo "Error: output file was not created" >&2
    exit 2
fi

SIZE=$(stat -f%z "$OUTPUT_PATH")
echo "Captured: $OUTPUT_PATH (${WIDTH}x${HEIGHT} region at ${X},${Y}, ${SIZE} bytes)"
