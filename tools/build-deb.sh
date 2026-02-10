#!/bin/bash
set -e

# Configuration
APP_NAME="packetparamedic"
VERSION=$(grep '^version =' Cargo.toml | cut -d '"' -f 2)
ARCH="aarch64"
TARGET="aarch64-unknown-linux-gnu"
BUILD_DIR="target/debian"
DEB_NAME="${APP_NAME}_${VERSION}_${ARCH}.deb"

echo "Building $APP_NAME version $VERSION for $ARCH..."

# Determine build command
BUILD_CMD="cargo"
if command -v cross &> /dev/null; then
    BUILD_CMD="cross"
fi

# Override with environment variable if set
if [ "$USE_CROSS" = "false" ]; then
    BUILD_CMD="cargo"
elif [ "$USE_CROSS" = "true" ]; then
    BUILD_CMD="cross"
fi

echo "Using build command: $BUILD_CMD"

# Build binary
echo "Compiling release binary..."
$BUILD_CMD build --release --target "$TARGET"

# verified binary exists
BINARY_PATH="target/$TARGET/release/$APP_NAME"
if [ ! -f "$BINARY_PATH" ]; then
    echo "Error: Binary not found at $BINARY_PATH"
    exit 1
fi

# Prepare directory structure
echo "Creating debian package structure..."
rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR/usr/local/bin"
mkdir -p "$BUILD_DIR/lib/systemd/system"
mkdir -p "$BUILD_DIR/etc/packetparamedic"
mkdir -p "$BUILD_DIR/usr/lib/tmpfiles.d"
mkdir -p "$BUILD_DIR/DEBIAN"

# Copy files
cp "$BINARY_PATH" "$BUILD_DIR/usr/local/bin/"
cp systemd/*.service "$BUILD_DIR/lib/systemd/system/"
cp systemd/*.conf "$BUILD_DIR/usr/lib/tmpfiles.d/"
cp config/*.toml "$BUILD_DIR/etc/packetparamedic/" 2>/dev/null || true

# Create control file
cat > "$BUILD_DIR/DEBIAN/control" <<EOF
Package: $APP_NAME
Version: $VERSION
Section: net
Priority: optional
Architecture: arm64
Maintainer: PacketParamedic Team
Description: Appliance-grade network diagnostics for Raspberry Pi 5
EOF

# Create postinst
cat > "$BUILD_DIR/DEBIAN/postinst" <<EOF
#!/bin/sh
set -e
# Create user if not exists
if ! id "packetparamedic" >/dev/null 2>&1; then
    useradd --system --no-create-home --group packetparamedic
fi
# Reload systemd
systemctl daemon-reload
# Enable service if installed for the first time
if [ "\$1" = "configure" ] && [ -z "\$2" ]; then
    systemctl enable packetparamedic
    systemctl start packetparamedic
fi
EOF
chmod 755 "$BUILD_DIR/DEBIAN/postinst"

# Build .deb
echo "Building .deb package..."
dpkg-deb --build "$BUILD_DIR" "target/$DEB_NAME"

echo "Done: target/$DEB_NAME"
