#!/bin/bash
set -e

# PacketParamedic - Automated Pre-requisites Setup
# Supports: Debian/Raspberry Pi OS (Client) and RHEL/AlmaLinux (Reflector Host)

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

log() { echo -e "${GREEN}[INFO]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
err() { echo -e "${RED}[ERROR]${NC} $1"; }

if [ "$EUID" -ne 0 ]; then
  err "Please run as root (sudo ./setup_prereqs.sh)"
  exit 1
fi

# Detect OS
if [ -f /etc/os-release ]; then
    . /etc/os-release
    OS=$NAME
else
    OS=$(uname -s)
fi

log "Detected OS: $OS"

setup_debian() {
    log "Update apt repositories..."
    apt-get update

    log "Installing Core Network Tools (iperf3, ethtool, mtr, iw)..."
    apt-get install -y \
        curl wget git build-essential \
        iperf3 ethtool iw mtr-tiny traceroute \
        iputils-ping jq \
        usbutils pciutils

    # 1. Ookla Speedtest
    if ! command -v speedtest &> /dev/null; then
        log "Installing Ookla Speedtest CLI..."
        curl -s https://packagecloud.io/install/repositories/ookla/speedtest-cli/script.deb.sh | bash
        apt-get install -y speedtest
    else
        log "Ookla Speedtest already installed."
    fi

    # 2. Node.js & Fast.com CLI (Optional)
    if ! command -v fast &> /dev/null; then
        log "Installing Node.js & fast-cli (Netflix Speed Test)..."
        # Using Node 18/20 LTS
        curl -fsSL https://deb.nodesource.com/setup_20.x | bash - || warn "Node setup script failed"
        apt-get install -y nodejs
        npm install --global fast-cli || warn "Failed to install fast-cli via npm"
    else
        log "fast-cli already installed."
    fi

    # 3. Go & NDT7 (Optional)
    if ! command -v ndt7-client &> /dev/null; then
        log "Installing Golang for NDT7..."
        apt-get install -y golang-go
        
        # Note: 'go install' usually targets user's GOPATH. Doing best effort system install.
        export GOPATH=/usr/local/go
        export PATH=$PATH:/usr/local/go/bin
        mkdir -p /usr/local/go
        
        log "Installing ndt7-client..."
        # This might fail if network is restricted or Go version is old on apt
        if go install github.com/m-lab/ndt7-client-go/cmd/ndt7-client@latest; then
             # Symlink to path if needed
             ln -sf /usr/local/go/bin/ndt7-client /usr/local/bin/ndt7-client
             log "NDT7 Installed."
        else
             warn "Failed to install NDT7 client. You may need a newer Go version."
        fi
    fi
}

setup_rhel() {
    log "Installing Server Essentials (EPEL, Tar, Git)..."
    dnf install -y epel-release
    dnf install -y iperf3 ethtool mtr traceroute bind-utils jq git wget curl tar

    # Podman for Reflector
    if ! command -v podman &> /dev/null; then
        log "Installing Podman (Required for Reflector)..."
        dnf install -y podman container-tools
    fi

    # Firewall Configuration (Reflector Ports)
    if systemctl is-active --quiet firewalld; then
        log "Opening Firewall for Reflector (Ports 4000, 5201-5210)..."
        firewall-cmd --permanent --add-port=4000/tcp
        firewall-cmd --permanent --add-port=5201-5210/tcp
        firewall-cmd --permanent --add-port=5201-5210/udp
        firewall-cmd --reload
    else
        warn "Firewalld not active. Ensure ports 4000 & 5201-5210 are open manually."
    fi
}

# Main Execution Switch
if [[ "$OS" == *"Debian"* ]] || [[ "$OS" == *"Ubuntu"* ]] || [[ "$OS" == *"Raspbian"* ]]; then
    setup_debian
elif [[ "$OS" == *"AlmaLinux"* ]] || [[ "$OS" == *"Red Hat"* ]] || [[ "$OS" == *"CentOS"* ]] || [[ "$OS" == *"Fedora"* ]]; then
    setup_rhel
else
    warn "Unsupported or Unknown OS: $OS"
    warn "Manual installation required for: iperf3, ethtool, speedtest, mtr"
fi

# Validation
log "--- Verification ---"
echo "iperf3: $(command -v iperf3 || echo 'MISSING')"
echo "ethtool: $(command -v ethtool || echo 'MISSING')"
echo "ookla: $(command -v speedtest || echo 'MISSING')"
echo "fast-cli: $(command -v fast || echo 'MISSING')"
log "Pre-requisites setup complete."
