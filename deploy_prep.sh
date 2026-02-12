#!/bin/bash
set -e

echo "=== PacketParamedic Prep Script ==="

# Already ran upgrade manually, but good to have apt update
sudo apt update

echo "Removing unnecessary desktop packages (bloat)..."
# Common bloat on Pi OS Desktop images
PKGS_TO_REMOVE="wolfram-engine minecraft-pi scratch scratch2 scratch3 sonic-pi dillo gpicview penguinspuzzle oracle-java8-jdk openjdk-11-jre openjdk-17-jre python-games python3-games libreoffice* chromium-browser* firefox* rpd-wallpaper"

# Only remove if installed to avoid errors or just use ignore failure
# We use || true to proceed even if some packages aren't found
sudo apt purge -y --auto-remove $PKGS_TO_REMOVE || echo "Some packages not found, proceeding..."

echo "Performing autoremove..."
sudo apt autoremove -y

echo "Installing dependencies..."
# build-essential, pkg-config,ssl, sqlite, git, curl
sudo apt install -y build-essential pkg-config libssl-dev sqlite3 git curl

# Headless OpenGL deps for Glutin/Glow
echo "Installing graphics dependencies for headless operation..."
sudo apt install -y libgl1-mesa-dev libgles2-mesa-dev libgbm-dev libegl1-mesa-dev libasound2-dev libudev-dev
# Winit/Glutin build deps (even for headless these are often needed for linking)
sudo apt install -y libx11-dev libxrandr-dev libxi-dev libxcursor-dev libxinerama-dev libxext-dev

echo "Installing Rust..."
if ! command -v cargo &> /dev/null; then
    echo "Installing Rust via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    # Source env for this session
    source "$HOME/.cargo/env"
else
    echo "Rust is already installed."
fi

echo "Activating UFW..."
sudo apt install -y ufw
# Reset ufw to defaults
echo "y" | sudo ufw reset
sudo ufw default deny incoming
sudo ufw default allow outgoing

# Allow SSH
sudo ufw allow 22/tcp
# Allow app port 8080 (axum default in src/main.rs)
sudo ufw allow 8080/tcp

echo "Enabling UFW..."
echo "y" | sudo ufw enable

echo "Hardening SSH..."
# Backup config
if [ ! -f /etc/ssh/sshd_config.bak ]; then
    sudo cp /etc/ssh/sshd_config /etc/ssh/sshd_config.bak
fi

# Ensure PermitRootLogin no
sudo sed -i 's/^PermitRootLogin.*/PermitRootLogin no/' /etc/ssh/sshd_config
if ! grep -q "^PermitRootLogin" /etc/ssh/sshd_config; then
    echo "PermitRootLogin no" | sudo tee -a /etc/ssh/sshd_config
fi

# Ensure PasswordAuthentication yes
sudo sed -i 's/^PasswordAuthentication.*/PasswordAuthentication yes/' /etc/ssh/sshd_config
if ! grep -q "^PasswordAuthentication" /etc/ssh/sshd_config; then
    echo "PasswordAuthentication yes" | sudo tee -a /etc/ssh/sshd_config
fi

# Reload SSH
sudo systemctl reload ssh

# Create data directory
mkdir -p ~/PacketParamedic/data

echo "Preparation complete! Ready to deploy."
