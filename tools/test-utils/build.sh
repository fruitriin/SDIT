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

echo "==> Building render-text..."
swiftc \
    "$SCRIPT_DIR/render-text.swift" \
    -o "$SCRIPT_DIR/render-text" \
    -framework CoreGraphics \
    -framework CoreText \
    -framework Foundation \
    -framework ImageIO
echo "    OK: $SCRIPT_DIR/render-text"

echo "==> Building verify-text..."
swiftc \
    "$SCRIPT_DIR/verify-text.swift" \
    -o "$SCRIPT_DIR/verify-text" \
    -framework CoreGraphics \
    -framework Vision \
    -framework ImageIO \
    -framework Foundation
echo "    OK: $SCRIPT_DIR/verify-text"

echo "==> Setting execute permission on send-keys.sh..."
chmod +x "$SCRIPT_DIR/send-keys.sh"
echo "    OK: $SCRIPT_DIR/send-keys.sh"

echo "==> Building annotate-grid..."
swiftc "$SCRIPT_DIR/annotate-grid.swift" -o "$SCRIPT_DIR/annotate-grid" \
    -framework CoreGraphics -framework CoreText -framework Foundation -framework ImageIO
echo "    OK: $SCRIPT_DIR/annotate-grid"

echo "==> Building clip-image..."
swiftc "$SCRIPT_DIR/clip-image.swift" -o "$SCRIPT_DIR/clip-image" \
    -framework CoreGraphics -framework Foundation -framework ImageIO
echo "    OK: $SCRIPT_DIR/clip-image"

echo ""
echo "Build complete. 次のステップ:"
echo "  1. System Settings → Privacy & Security → Screen Recording"
echo "     で window-info と capture-window を許可"
echo "  2. 許可後に OS を再起動"
echo "  3. cargo test --test gui_interaction -- --ignored で GUI テストを実行"
