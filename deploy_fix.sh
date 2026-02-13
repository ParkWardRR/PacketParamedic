#!/bin/bash
set -e

# Configuration
SERVER_USER="alfa"
SERVER_HOST="irww.alpina"
CLIENT_USER="alfa" 
CLIENT_HOST="packetparamedic.alpina"
REMOTE_DIR="~/PacketParamedic"

echo "=== Deploying Reflector Fixes (Production) ==="

# 1. Server Deployment (IRWW - AlmaLinux/Podman Quadlet)
echo "--> Syncing ENTIRE REPO to ${SERVER_USER}@${SERVER_HOST}..."
# Sync everything (including src, Cargo.toml) so container build has full context
rsync -avz --exclude 'target' --exclude '.git' ./ ${SERVER_USER}@${SERVER_HOST}:$REMOTE_DIR/

echo "--> Configuring Server (Podman Quadlet) on ${SERVER_USER}@${SERVER_HOST}..."
ssh -T ${SERVER_USER}@${SERVER_HOST} <<ENDSSH
    set -e
    REMOTE_DIR="\$HOME/PacketParamedic"
    
    # 0. Setup Environment (AlmaLinux)
    # Ensure Docker is gone and Podman is present
    if command -v dnf &> /dev/null; then
        echo "Detected AlmaLinux/RHEL. ensuring Podman..."
        sudo dnf remove -y docker docker-ce docker-ce-cli || true
        sudo dnf install -y podman container-tools || true
    fi
    
    # 1. Update Config (require sudo)
    # We assume 'alfa' has sudo NOPASSWD or you will get prompts?
    # Interactive sudo in pipeline might fail.
    # But user says "use passwordless".
    
    sudo mkdir -p /etc/reflector
    sudo mkdir -p /var/lib/reflector
    sudo chown -R \$(whoami) /etc/reflector /var/lib/reflector

    if [ ! -f /etc/reflector/reflector.toml ]; then
        echo 'Config missing, installing default...'
        # Copy from repo path
        cp \$REMOTE_DIR/reflector/local.config.toml /etc/reflector/reflector.toml
    fi

    # Ensure Quotas
    if grep -q 'max_concurrent_tests' /etc/reflector/reflector.toml; then
        echo 'Quotas already configured.'
    else
        echo 'Appending quotas...'
        echo '' >> /etc/reflector/reflector.toml
        echo '[quotas]' >> /etc/reflector/reflector.toml
        echo 'max_concurrent_tests = 4' >> /etc/reflector/reflector.toml
    fi

    # 2. Prepare Podman Quadlet
    mkdir -p ~/.config/containers/systemd
    cat <<EOF > ~/.config/containers/systemd/reflector.container
[Unit]
Description=PacketParamedic Reflector Service
After=network-online.target

[Container]
Image=reflector:latest
ContainerName=reflector
Network=host
User=0
Volume=/etc/reflector/reflector.toml:/etc/reflector/reflector.toml:ro,z
Volume=/var/lib/reflector:/var/lib/reflector:z

[Service]
Restart=always
TimeoutStartSec=900

[Install]
WantedBy=default.target
EOF

    # 3. Build Image & Start Service
    echo "Building image..."
    cd \$REMOTE_DIR
    podman build -f reflector/Containerfile.local -t reflector:latest .
    
    echo "Reloading systemd..."
    systemctl --user daemon-reload
    systemctl --user restart reflector
    
    loginctl enable-linger \$(whoami) || true
    
    echo "Service Status:"
    systemctl --user status reflector --no-pager
ENDSSH

# 2. Client Deployment (PacketParamedic - Pi/Native)
echo "=== Deploying Client Fixes ==="
echo "--> Syncing Client source to ${CLIENT_USER}@${CLIENT_HOST}..."
rsync -avz --exclude 'target' --exclude '.git' src/ ${CLIENT_USER}@${CLIENT_HOST}:$REMOTE_DIR/src/
rsync -avz Cargo.toml ${CLIENT_USER}@${CLIENT_HOST}:$REMOTE_DIR/
rsync -avz Cargo.lock ${CLIENT_USER}@${CLIENT_HOST}:$REMOTE_DIR/

echo "--> Rebuilding PacketParamedic Client on ${CLIENT_USER}@${CLIENT_HOST}..."
ssh ${CLIENT_USER}@${CLIENT_HOST} "cd $REMOTE_DIR && \
    source ~/.cargo/env || true && \
    cargo build --release --bin packetparamedic && \
    echo 'Client build complete. Restart your service if applicable.'"

# 3. Local OrbStack Deployment (Mac)
echo "=== Deploying Local Reflector (OrbStack) ==="
echo "--> Building Reflector Docker image locally..."
docker build -f reflector/Containerfile.local -t reflector:latest .

echo "--> Updating local config..."
# Ensure quotas are set locally too
if ! grep -q 'max_concurrent_tests' reflector/local.config.toml; then
    echo '' >> reflector/local.config.toml
    echo '[quotas]' >> reflector/local.config.toml
    echo 'max_concurrent_tests = 4' >> reflector/local.config.toml
fi

echo "--> Restarting Local Reflector container..."
docker rm -f reflector || true
docker run -d --name reflector \
    --restart unless-stopped \
    -p 4000:4000 -p 5201-5210:5201-5210 \
    -v $(pwd)/reflector/local.config.toml:/etc/reflector/reflector.toml \
    -v $(pwd)/reflector/deploy/reflector.toml:/etc/reflector/defaults.toml \
    reflector:latest serve

echo "=== Deployment Complete ==="
