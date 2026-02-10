#!/bin/bash
set -e

# Configuration
IMAGE_NAME="packetparamedic-builder"
DOCKERFILE="tools/Dockerfile.build"

echo "Building docker image..."
docker build -t "$IMAGE_NAME" -f "$DOCKERFILE" .

echo "Running build inside container..."
docker run --rm \
    -v "$(pwd):/app" \
    -e USE_CROSS=false \
    "$IMAGE_NAME" ./tools/build-deb.sh

echo "Build complete."
