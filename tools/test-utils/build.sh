#!/bin/bash
# build.sh
#
# Swift スクリプトをコンパイルしてバイナリを生成する。
#
# Usage: ./build.sh
# 出力先: tools/test-utils/ (このスクリプトと同じディレクトリ)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "==> Building window-info..."
swiftc \
    "$SCRIPT_DIR/window-info.swift" \
    -o "$SCRIPT_DIR/window-info" \
    -framework ApplicationServices \
    -framework Foundation
echo "    OK: $SCRIPT_DIR/window-info"

echo "==> Building capture-window..."
swiftc \
    "$SCRIPT_DIR/capture-window.swift" \
    -o "$SCRIPT_DIR/capture-window" \
    -framework ScreenCaptureKit \
    -framework CoreGraphics \
    -framework Foundation
echo "    OK: $SCRIPT_DIR/capture-window"

echo "==> Setting execute permission on send-keys.sh..."
chmod +x "$SCRIPT_DIR/send-keys.sh"
echo "    OK: $SCRIPT_DIR/send-keys.sh"

echo ""
echo "Build complete. 次のステップ:"
echo "  1. System Settings → Privacy & Security → Screen Recording"
echo "     で window-info と capture-window を許可"
echo "  2. 許可後に OS を再起動"
echo "  3. cargo test --test gui_interaction -- --ignored で GUI テストを実行"
