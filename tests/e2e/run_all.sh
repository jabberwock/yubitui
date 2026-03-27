#!/usr/bin/env bash
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/../.."  # project root

# Build once for all tests
echo "Building yubitui..."
cargo build --quiet 2>&1 || { echo "FAIL: cargo build failed"; exit 1; }
export BINARY="$(pwd)/target/debug/yubitui"

PASS=0
FAIL=0
FAILED_TESTS=""

for script in "$SCRIPT_DIR"/*_smoke.sh; do
    test_name=$(basename "$script" .sh)
    echo "Running: $test_name..."
    if bash "$script"; then
        PASS=$((PASS + 1))
    else
        FAIL=$((FAIL + 1))
        FAILED_TESTS="$FAILED_TESTS $test_name"
    fi
done

echo ""
echo "================================"
echo "E2E Results: $PASS passed, $FAIL failed"
if [ "$FAIL" -gt 0 ]; then
    echo "Failed:$FAILED_TESTS"
fi
echo "================================"
[ "$FAIL" -eq 0 ]
