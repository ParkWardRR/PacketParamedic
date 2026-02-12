#!/bin/bash
set -e

# Configuration
# Hardcoded to a known stable version for ARM64 (Verification: Aug 2024)
RUNNER_VERSION="2.319.1"
RUNNER_DIR=~/actions-runner
REPO_URL="https://github.com/ParkWardRR/PacketParamedic"

echo "=== PacketParamedic Pi 5 GitHub Runner Setup ==="

# Check/Prompt for Token
if [ -z "$1" ]; then
    echo "Error: GitHub Runner Token is required."
    echo "1. Go to: $REPO_URL/settings/actions/runners/new"
    echo "2. Generate a token (Architecture: Linux, ARM64)"
    echo "3. Run this script: ./tools/setup-runner.sh <TOKEN>"
    read -p "Or enter token now: " TOKEN
else
    TOKEN="$1"
fi

if [ -z "$TOKEN" ]; then
    echo "No token provided. Exiting."
    exit 1
fi

# 1. Prepare Directory
echo "[-] Creating runner directory at $RUNNER_DIR..."
mkdir -p "$RUNNER_DIR"
cd "$RUNNER_DIR"

# 2. Download Runner (idempotent)
if [ ! -f "config.sh" ]; then
    echo "[-] Downloading actions-runner-linux-arm64-${RUNNER_VERSION}..."
    curl -o actions-runner-linux-arm64-${RUNNER_VERSION}.tar.gz -L https://github.com/actions/runner/releases/download/v${RUNNER_VERSION}/actions-runner-linux-arm64-${RUNNER_VERSION}.tar.gz
    
    echo "[-] Extracting runner..."
    tar xzf ./actions-runner-linux-arm64-${RUNNER_VERSION}.tar.gz
else
    echo "[-] Runner already downloaded."
fi

# 3. Install Dependencies
echo "[-] Installing .NET Core dependencies..."
# This script installs libicu, libssl, etc. required by the runner
sudo ./bin/installdependencies.sh

# 4. Configure Runner
# remove existing config if present to allow re-registration
if [ -f ".runner" ]; then
    echo "[-] Removing existing runner configuration..."
    ./config.sh remove --token "$TOKEN" || true
fi

echo "[-] Configuring runner..."
# Utilizing --replace in case of hostname collision
./config.sh --url "$REPO_URL" \
    --token "$TOKEN" \
    --name "$(hostname)-pi5" \
    --labels "self-hosted,linux,arm64,pi5" \
    --work "_work" \
    --unattended \
    --replace

# 5. Install & Start Service
echo "[-] Installing systemd service..."
sudo ./svc.sh install || echo "Service might already be installed."
sudo ./svc.sh start || echo "Service might already be running."

echo "=== SUCCESS ==="
echo "Runner installed and started."
sudo ./svc.sh status
