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

echo "==> savanna-smell-detector (severity >= 4)"
savanna-smell-detector --min-severity 4 --fail-on-smell crates/

echo ""
echo "All checks passed."
