#!/bin/bash
# check.sh — テストチェーン
#
# cargo fmt/clippy/test + savanna-smell-detector を一括実行する。
# CI やコミット前の確認に使用。
#
# Usage: ./scripts/check.sh

set -euo pipefail

echo "==> cargo fmt --check"
cargo fmt --check

echo "==> cargo clippy --all-targets"
cargo clippy --all-targets

echo "==> cargo test"
cargo test

echo "==> savanna-smell-detector (severity >= 1)"
# ターミナルプロジェクト固有のマジックナンバーホワイトリスト
# 24,80: 標準ターミナルサイズ  0,1: 境界値  255,256: 8bit境界  4096: バッファ上限
SMELL_MAGIC_WHITELIST="24,80,0,1,255,256,4096"
savanna-smell-detector --min-severity 1 --fail-on-smell \
  --magic-number-whitelist "$SMELL_MAGIC_WHITELIST" \
  --assertion-roulette-threshold 5 crates/

echo ""
echo "All checks passed."
