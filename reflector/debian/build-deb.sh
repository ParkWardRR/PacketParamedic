#!/bin/bash
# build-deb.sh - Build a .deb package for the PacketParamedic Reflector
# Target: Debian / Ubuntu (amd64)
#
# Usage:
#   cd <repo-root>/reflector
#   ./debian/build-deb.sh
#
# Prerequisites:
#   - Rust toolchain (rustup + cargo)
#   - dpkg-deb (usually pre-installed on Debian/Ubuntu)
#   - Optional: cross (for cross-compilation from non-amd64 hosts)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REFLECTOR_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd "$REFLECTOR_DIR/.." && pwd)"

# Configuration
APP_NAME="reflector"
VERSION=$(grep '^version' "$REFLECTOR_DIR/Cargo.toml" | head -1 | cut -d '"' -f 2)
ARCH="amd64"
TARGET="x86_64-unknown-linux-gnu"
BUILD_DIR="$REFLECTOR_DIR/target/debian"
DEB_NAME="${APP_NAME}_${VERSION}_${ARCH}.deb"

echo "=== Building $APP_NAME v$VERSION .deb for $ARCH ==="

# ---------------------------------------------------------------------------
# Step 1: Determine build command (cargo or cross)
# ---------------------------------------------------------------------------
BUILD_CMD="cargo"
if command -v cross &>/dev/null; then
    BUILD_CMD="cross"
fi

# Override with environment variable if set
if [ "${USE_CROSS:-}" = "false" ]; then
    BUILD_CMD="cargo"
elif [ "${USE_CROSS:-}" = "true" ]; then
    BUILD_CMD="cross"
fi

echo "--- Using build command: $BUILD_CMD ---"

# ---------------------------------------------------------------------------
# Step 2: Build the binary with AVX2 optimization (N100 target)
# ---------------------------------------------------------------------------
echo "--- Compiling release binary ---"
cd "$REPO_ROOT"
RUSTFLAGS="-C target-cpu=x86-64-v3" $BUILD_CMD build \
    --release \
    --target "$TARGET" \
    --manifest-path reflector/Cargo.toml

BINARY_PATH="$REPO_ROOT/target/$TARGET/release/$APP_NAME"
if [ ! -f "$BINARY_PATH" ]; then
    # Fallback: check without target triple (native builds)
    BINARY_PATH="$REPO_ROOT/target/release/$APP_NAME"
fi

if [ ! -f "$BINARY_PATH" ]; then
    echo "ERROR: Binary not found"
    exit 1
fi

# Strip debug symbols
strip "$BINARY_PATH" 2>/dev/null || true

echo "Binary size: $(du -h "$BINARY_PATH" | cut -f1)"

# ---------------------------------------------------------------------------
# Step 3: Create debian package structure
# ---------------------------------------------------------------------------
echo "--- Creating debian package structure ---"
rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR/usr/local/bin"
mkdir -p "$BUILD_DIR/lib/systemd/system"
mkdir -p "$BUILD_DIR/etc/reflector"
mkdir -p "$BUILD_DIR/DEBIAN"

# ---------------------------------------------------------------------------
# Step 4: Copy files into package tree
# ---------------------------------------------------------------------------
echo "--- Copying files ---"
cp "$BINARY_PATH"                                  "$BUILD_DIR/usr/local/bin/reflector"
cp "$REFLECTOR_DIR/systemd/reflector.service"      "$BUILD_DIR/lib/systemd/system/"
cp "$REFLECTOR_DIR/deploy/reflector.toml.example"  "$BUILD_DIR/etc/reflector/reflector.toml"

# ---------------------------------------------------------------------------
# Step 5: Write DEBIAN/control
# ---------------------------------------------------------------------------
cat > "$BUILD_DIR/DEBIAN/control" <<EOF
Package: $APP_NAME
Version: $VERSION
Section: net
Priority: optional
Architecture: $ARCH
Depends: iperf3
Maintainer: PacketParamedic Team <dev@packetparamedic.io>
Homepage: https://github.com/packetparamedic/packetparamedic
License: BlueOak-1.0.0
Description: PacketParamedic Reflector - Self-hosted Network Test Endpoint
 A single-binary, cryptographically-identified endpoint that exposes a
 zero-trust control plane and a tightly-scoped data plane (throughput +
 latency reflector). Designed to be safe to run on the public Internet
 without becoming an open relay.
EOF

# ---------------------------------------------------------------------------
# Step 6: Copy maintainer scripts
# ---------------------------------------------------------------------------
cp "$REFLECTOR_DIR/debian/postinst" "$BUILD_DIR/DEBIAN/postinst"
cp "$REFLECTOR_DIR/debian/prerm"    "$BUILD_DIR/DEBIAN/prerm"
chmod 755 "$BUILD_DIR/DEBIAN/postinst"
chmod 755 "$BUILD_DIR/DEBIAN/prerm"

# ---------------------------------------------------------------------------
# Step 7: Set permissions
# ---------------------------------------------------------------------------
chmod 755 "$BUILD_DIR/usr/local/bin/reflector"
chmod 644 "$BUILD_DIR/lib/systemd/system/reflector.service"
chmod 644 "$BUILD_DIR/etc/reflector/reflector.toml"

# ---------------------------------------------------------------------------
# Step 8: Build the .deb
# ---------------------------------------------------------------------------
echo "--- Building .deb package ---"
dpkg-deb --build --root-owner-group "$BUILD_DIR" "$REFLECTOR_DIR/target/$DEB_NAME"

echo ""
echo "=== .deb build complete ==="
echo "  Package: $REFLECTOR_DIR/target/$DEB_NAME"
echo "  Size:    $(du -h "$REFLECTOR_DIR/target/$DEB_NAME" | cut -f1)"
echo ""
echo "Install with:"
echo "  sudo dpkg -i $REFLECTOR_DIR/target/$DEB_NAME"
echo "  sudo apt-get install -f  # resolve dependencies if needed"
echo ""
echo "Or copy to target host and install:"
echo "  scp $REFLECTOR_DIR/target/$DEB_NAME user@host:/tmp/"
echo "  ssh user@host 'sudo apt install /tmp/$DEB_NAME'"
