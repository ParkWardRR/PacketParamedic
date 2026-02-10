#!/usr/bin/env bash
# PacketParamedic -- Local CI script
# Run this before pushing. It does what GitHub CI would do, locally.
#
# Usage: ./tools/ci-local.sh [--quick]
#   --quick  Skip slow checks (audit, build release)

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

pass() { echo -e "${GREEN}PASS${NC} $1"; }
fail() { echo -e "${RED}FAIL${NC} $1"; exit 1; }
skip() { echo -e "${YELLOW}SKIP${NC} $1"; }

QUICK=false
if [[ "${1:-}" == "--quick" ]]; then
    QUICK=true
fi

echo "=== PacketParamedic Local CI ==="
echo ""

# 1. Format check
echo "--- cargo fmt --check ---"
cargo fmt --check && pass "formatting" || fail "formatting (run: cargo fmt)"

# 2. Clippy
echo "--- cargo clippy ---"
cargo clippy -- -D warnings && pass "clippy" || fail "clippy"

# 3. Tests
echo "--- cargo test ---"
cargo test && pass "tests" || fail "tests"

# 4. Audit (slow -- skip in quick mode)
if $QUICK; then
    skip "cargo audit (quick mode)"
else
    echo "--- cargo audit ---"
    if command -v cargo-audit &> /dev/null; then
        cargo audit && pass "audit" || fail "audit"
    else
        skip "cargo audit (not installed: cargo install cargo-audit)"
    fi
fi

# 5. Build release (slow -- skip in quick mode)
if $QUICK; then
    skip "release build (quick mode)"
else
    echo "--- cargo build --release ---"
    cargo build --release && pass "release build" || fail "release build"
fi

echo ""
echo "=== All checks passed ==="
