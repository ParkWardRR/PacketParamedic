#!/bin/bash
# build-rpm.sh - Build an RPM package for the PacketParamedic Reflector
# Target: Alma Linux / RHEL / Fedora (x86_64)
#
# Usage:
#   cd <repo-root>/reflector
#   ./rpm/build-rpm.sh
#
# Prerequisites:
#   - Rust toolchain (rustup + cargo)
#   - rpm-build package: dnf install rpm-build
#   - rpmlint (optional): dnf install rpmlint
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REFLECTOR_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd "$REFLECTOR_DIR/.." && pwd)"

# Extract version from Cargo.toml
VERSION=$(grep '^version' "$REFLECTOR_DIR/Cargo.toml" | head -1 | cut -d '"' -f 2)
NAME="reflector"

echo "=== Building $NAME v$VERSION RPM for x86_64 ==="

# ---------------------------------------------------------------------------
# Step 1: Build the reflector binary with AVX2 flags (N100 target)
# ---------------------------------------------------------------------------
echo "--- Compiling release binary with AVX2 optimization ---"
cd "$REPO_ROOT"
RUSTFLAGS="-C target-cpu=x86-64-v3" cargo build \
    --release \
    --manifest-path reflector/Cargo.toml

BINARY_PATH="$REPO_ROOT/target/release/reflector"
if [ ! -f "$BINARY_PATH" ]; then
    echo "ERROR: Binary not found at $BINARY_PATH"
    exit 1
fi

# Strip debug symbols (in case profile.release.strip wasn't applied)
strip "$BINARY_PATH" 2>/dev/null || true

echo "Binary size: $(du -h "$BINARY_PATH" | cut -f1)"

# ---------------------------------------------------------------------------
# Step 2: Create RPM build directory structure
# ---------------------------------------------------------------------------
echo "--- Setting up rpmbuild tree ---"
RPM_TOPDIR="$REFLECTOR_DIR/target/rpmbuild"
rm -rf "$RPM_TOPDIR"
mkdir -p "$RPM_TOPDIR"/{BUILD,RPMS,SOURCES,SPECS,SRPMS}

# ---------------------------------------------------------------------------
# Step 3: Copy source files into SOURCES
# ---------------------------------------------------------------------------
echo "--- Copying sources ---"
cp "$BINARY_PATH"                                "$RPM_TOPDIR/SOURCES/reflector"
cp "$REFLECTOR_DIR/systemd/reflector.service"    "$RPM_TOPDIR/SOURCES/reflector.service"
cp "$REFLECTOR_DIR/deploy/reflector.toml.example" "$RPM_TOPDIR/SOURCES/reflector.toml.example"

# Copy spec file
cp "$REFLECTOR_DIR/rpm/reflector.spec"           "$RPM_TOPDIR/SPECS/reflector.spec"

# ---------------------------------------------------------------------------
# Step 4: Build the RPM
# ---------------------------------------------------------------------------
echo "--- Running rpmbuild ---"
rpmbuild \
    --define "_topdir $RPM_TOPDIR" \
    --define "version $VERSION" \
    -bb "$RPM_TOPDIR/SPECS/reflector.spec"

# ---------------------------------------------------------------------------
# Step 5: Report output
# ---------------------------------------------------------------------------
RPM_PATH=$(find "$RPM_TOPDIR/RPMS" -name "*.rpm" -type f | head -1)

if [ -z "$RPM_PATH" ]; then
    echo "ERROR: RPM build failed - no .rpm file found"
    exit 1
fi

echo ""
echo "=== RPM build complete ==="
echo "  Package: $RPM_PATH"
echo "  Size:    $(du -h "$RPM_PATH" | cut -f1)"
echo ""
echo "Install with:"
echo "  sudo dnf install $RPM_PATH"
echo ""
echo "Or copy to target host and install:"
echo "  scp $RPM_PATH user@host:/tmp/"
echo "  ssh user@host 'sudo dnf install /tmp/$(basename "$RPM_PATH")'"
