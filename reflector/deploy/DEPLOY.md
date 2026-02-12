# PacketParamedic Reflector -- Deployment Guide

This guide covers all supported deployment methods for the PacketParamedic
Reflector.

---

## Table of Contents

- [1. Podman Quadlet (Primary -- Alma Linux / RHEL)](#1-podman-quadlet-primary----alma-linux--rhel)
- [2. Docker Compose](#2-docker-compose)
- [3. Kubernetes](#3-kubernetes)
- [4. OrbStack (macOS)](#4-orbstack-macos)
- [5. Bare Metal (RPM)](#5-bare-metal-rpm)
- [6. Bare Metal (Debian/Ubuntu)](#6-bare-metal-debianubuntu)
- [7. Firewall Configuration](#7-firewall-configuration)
- [8. Post-Installation](#8-post-installation)

---

## 1. Podman Quadlet (Primary -- Alma Linux / RHEL)

Podman Quadlet is the recommended deployment method for production. It uses
systemd to manage the container lifecycle, provides automatic restarts, and
integrates with `journalctl` for logging.

### 1.1 Install Podman

```bash
# Alma Linux 9 / RHEL 9
sudo dnf install -y podman

# Verify installation
podman --version
# Expected: podman version 4.x or later
```

### 1.2 Build the Container Image

```bash
# Clone the repository
git clone https://github.com/packetparamedic/london-v1.git
cd london-v1

# Build the container image
podman build -f reflector/Containerfile -t reflector:latest .

# Verify the image
podman images | grep reflector
```

### 1.3 Create the Configuration

```bash
# Create the configuration directory
sudo mkdir -p /etc/reflector

# Create the configuration file
sudo tee /etc/reflector/reflector.toml << 'EOF'
[identity]
private_key_path = "/var/lib/reflector/identity.ed25519"

[network]
listen_address = "0.0.0.0:4000"
mode = "tunneled"
data_port_range_start = 5201
data_port_range_end = 5210

[access]
pairing_enabled = false
authorized_peers = []

[quotas]
max_test_duration_sec = 60
max_concurrent_tests = 1
max_tests_per_hour_per_peer = 10
max_bytes_per_day_per_peer = 5000000000
cooldown_sec = 5
allow_udp_echo = true
allow_throughput = true

[iperf3]
path = "iperf3"
default_streams = 4
max_streams = 8

[logging]
level = "info"
audit_log_path = "/var/lib/reflector/audit.jsonl"
EOF
```

### 1.4 Copy Quadlet Files

Create the Quadlet unit file for systemd:

```bash
# Create the Quadlet directory for rootless Podman
mkdir -p ~/.config/containers/systemd

# Create the container unit file
cat > ~/.config/containers/systemd/reflector.container << 'EOF'
[Unit]
Description=PacketParamedic Reflector
After=network-online.target
Wants=network-online.target

[Container]
Image=localhost/reflector:latest
PublishPort=4000:4000
PublishPort=5201-5210:5201-5210
Volume=reflector-data:/var/lib/reflector
Volume=/etc/reflector/reflector.toml:/etc/reflector/reflector.toml:ro
SecurityLabelDisable=true
NoNewPrivileges=true
ReadOnly=true
DropCapability=ALL
AutoUpdate=local
HealthCmd=/usr/local/bin/reflector status
HealthInterval=30s
HealthTimeout=5s
HealthStartPeriod=10s

[Service]
Restart=always
RestartSec=10
TimeoutStartSec=90

[Install]
WantedBy=default.target
EOF
```

### 1.5 Start with systemd

```bash
# Reload systemd to pick up the new Quadlet file
systemctl --user daemon-reload

# Start the reflector
systemctl --user start reflector

# Enable auto-start on login
systemctl --user enable reflector

# Verify it is running
systemctl --user status reflector
```

### 1.6 Verify

```bash
# Check the Endpoint ID
podman exec systemd-reflector reflector show-id

# Check status
podman exec systemd-reflector reflector status

# Verify the port is listening
ss -tlnp sport eq 4000
```

### 1.7 Managing Peers (Pairing)

```bash
# Enable pairing temporarily
podman exec systemd-reflector reflector pair --ttl 10m

# Note the Endpoint ID and pairing token.
# Share these with the PacketParamedic appliance operator.

# After the appliance pairs, you can verify the connection in the audit log:
podman exec systemd-reflector cat /var/lib/reflector/audit.jsonl | \
  jq 'select(.event_type == "peer_paired")'
```

Alternatively, pre-authorize peers in the configuration file:

```toml
[access]
authorized_peers = [
    "PP-AAAA-BBBB-CCCC-0",
    "PP-DDDD-EEEE-FFFF-1",
]
```

Then restart: `systemctl --user restart reflector`

### 1.8 Viewing Logs

```bash
# Follow the container logs via journalctl
journalctl --user -u reflector -f

# View the last 100 log lines
journalctl --user -u reflector -n 100

# View the structured audit log
podman exec systemd-reflector cat /var/lib/reflector/audit.jsonl | jq .
```

### 1.9 Updating

```bash
# Rebuild the image
cd london-v1
git pull
podman build -f reflector/Containerfile -t reflector:latest .

# Restart the service (Quadlet auto-update can also handle this)
systemctl --user restart reflector

# Or use Podman auto-update (if AutoUpdate=local is set)
podman auto-update
```

---

## 2. Docker Compose

### 2.1 Install Docker

Follow the official Docker installation guide for your platform:
https://docs.docker.com/engine/install/

### 2.2 Clone and Configure

```bash
git clone https://github.com/packetparamedic/london-v1.git
cd london-v1/reflector/deploy

# Create the configuration file
cp reflector.toml.example reflector.toml
# Edit reflector.toml as needed (see Configuration Reference in README.md)
```

If `reflector.toml.example` does not exist, create `reflector.toml`:

```bash
cat > reflector.toml << 'EOF'
[identity]
private_key_path = "/var/lib/reflector/identity.ed25519"

[network]
listen_address = "0.0.0.0:4000"
mode = "tunneled"

[access]
pairing_enabled = false

[quotas]
max_test_duration_sec = 60
max_concurrent_tests = 1
max_tests_per_hour_per_peer = 10
cooldown_sec = 5

[logging]
level = "info"
audit_log_path = "/var/lib/reflector/audit.jsonl"
EOF
```

### 2.3 Start the Service

```bash
# Build and start
docker compose up -d

# Or build explicitly first
docker compose build
docker compose up -d
```

### 2.4 Verify Health

```bash
# Check container status
docker compose ps

# Check logs
docker compose logs reflector

# Verify the Endpoint ID
docker compose exec reflector reflector show-id

# Check the health status
docker inspect --format='{{.State.Health.Status}}' \
  $(docker compose ps -q reflector)
```

### 2.5 Configuration

The `docker-compose.yml` mounts the configuration file as read-only:

```yaml
volumes:
  - reflector-data:/var/lib/reflector
  - ./reflector.toml:/etc/reflector/reflector.toml:ro
```

Edit `reflector.toml` and restart:

```bash
docker compose restart reflector
```

### 2.6 Logs

```bash
# Follow logs in real-time
docker compose logs -f reflector

# View audit log
docker compose exec reflector cat /var/lib/reflector/audit.jsonl | jq .
```

### 2.7 Security Features

The `docker-compose.yml` includes security hardening:

```yaml
security_opt:
  - no-new-privileges:true   # Prevent privilege escalation
read_only: true               # Read-only root filesystem
cap_drop:
  - ALL                       # Drop all Linux capabilities
```

---

## 3. Kubernetes

### 3.1 Apply Manifests

The Kubernetes manifests are in `reflector/deploy/k8s/`.

```bash
# Create the namespace (optional)
kubectl create namespace packetparamedic

# Create the ConfigMap from your configuration
kubectl -n packetparamedic create configmap reflector-config \
  --from-file=reflector.toml=/path/to/your/reflector.toml

# Create a PersistentVolumeClaim for identity and audit data
kubectl -n packetparamedic apply -f - << 'EOF'
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: reflector-data
spec:
  accessModes:
    - ReadWriteOnce
  resources:
    requests:
      storage: 1Gi
EOF

# Apply the Deployment
kubectl -n packetparamedic apply -f reflector/deploy/k8s/deployment.yaml

# Apply the Service
kubectl -n packetparamedic apply -f reflector/deploy/k8s/service.yaml
```

### 3.2 Configure via ConfigMap

```bash
# Update the configuration
kubectl -n packetparamedic create configmap reflector-config \
  --from-file=reflector.toml=/path/to/updated/reflector.toml \
  --dry-run=client -o yaml | kubectl apply -f -

# Restart the deployment to pick up config changes
kubectl -n packetparamedic rollout restart deployment/reflector
```

### 3.3 Expose via Service/Ingress

The default Service type is `ClusterIP`. For external access:

```bash
# Option A: NodePort
kubectl -n packetparamedic patch svc reflector \
  -p '{"spec": {"type": "NodePort"}}'

# Option B: LoadBalancer (cloud providers)
kubectl -n packetparamedic patch svc reflector \
  -p '{"spec": {"type": "LoadBalancer"}}'

# Option C: Ingress (for TLS passthrough -- NOT recommended since
# the reflector handles its own TLS). If you must:
kubectl -n packetparamedic apply -f - << 'EOF'
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: reflector
  annotations:
    nginx.ingress.kubernetes.io/ssl-passthrough: "true"
spec:
  rules:
    - host: reflector.example.com
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: reflector
                port:
                  number: 4000
EOF
```

**Important:** The reflector manages its own TLS (mTLS with mutual
authentication). Do NOT terminate TLS at the ingress level. Use TLS passthrough
if you need an ingress controller.

### 3.4 Persistent Storage

The Deployment mounts two volumes:

- `reflector-data` at `/var/lib/reflector` -- Identity key and audit log
- `reflector-config` at `/etc/reflector` -- Configuration file (read-only)

The identity key is generated on first startup and stored in the PVC. The PVC
must survive pod restarts to maintain the same Endpoint ID.

### 3.5 Monitoring

```bash
# Check pod status
kubectl -n packetparamedic get pods -l app.kubernetes.io/name=reflector

# View logs
kubectl -n packetparamedic logs -f deployment/reflector

# Check liveness and readiness probes
kubectl -n packetparamedic describe pod -l app.kubernetes.io/name=reflector

# Get the Endpoint ID
kubectl -n packetparamedic exec deployment/reflector -- reflector show-id
```

### 3.6 Resource Limits

The Deployment includes sensible resource requests and limits:

```yaml
resources:
  requests:
    memory: "64Mi"
    cpu: "100m"
  limits:
    memory: "256Mi"
    cpu: "500m"
```

Adjust these based on your expected load. For throughput testing, you may need
to increase CPU limits.

### 3.7 Security Context

The Deployment runs with a restrictive security context:

```yaml
securityContext:
  runAsNonRoot: true
  runAsUser: 65534            # nobody
  readOnlyRootFilesystem: true
  allowPrivilegeEscalation: false
  capabilities:
    drop: [ALL]
```

---

## 4. OrbStack (macOS)

OrbStack is a lightweight container runtime for macOS that is faster and lighter
than Docker Desktop.

### 4.1 Install OrbStack

Download from https://orbstack.dev/ or:

```bash
brew install orbstack
```

### 4.2 Use Docker Compose

OrbStack is Docker-compatible, so the Docker Compose workflow works unchanged:

```bash
cd reflector/deploy
docker compose up -d
docker compose exec reflector reflector show-id
```

### 4.3 Use an OrbStack VM

For a more production-like environment, create a Linux VM:

```bash
# Create an Alma Linux VM
orb create alma reflector-dev

# SSH into the VM
orb shell reflector-dev

# Inside the VM: install dependencies and build
sudo dnf install -y gcc make iperf3
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
cd /path/to/reflector
cargo build --release
./target/release/reflector serve
```

### 4.4 Port Forwarding

OrbStack automatically forwards container ports to the macOS host. If running
the reflector in a Docker container:

```bash
docker run -d --name reflector \
  -p 4000:4000 \
  -p 5201-5210:5201-5210 \
  -v reflector-data:/var/lib/reflector \
  reflector:latest

# Access from macOS
nc -zv localhost 4000
```

### 4.5 Development Workflow

For rapid iteration on macOS:

```bash
# Build natively (no container overhead)
cargo build

# Run with a local test config
./target/debug/reflector -c test-config.toml serve

# Test with the built-in test suite
cargo test

# When ready for container testing:
docker build -f Containerfile -t reflector:dev ..
docker run --rm -p 4000:4000 reflector:dev
```

---

## 5. Bare Metal (RPM)

For Alma Linux / RHEL / Fedora systems without containers.

### 5.1 Build the RPM

```bash
# Ensure build dependencies are installed
sudo dnf install -y gcc make rpm-build iperf3

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Build the release binary
cd reflector
RUSTFLAGS="-C target-cpu=x86-64-v3" cargo build --release
strip target/release/reflector

# Build the RPM (if build script exists)
# ./rpm/build-rpm.sh

# Or package manually:
mkdir -p ~/rpmbuild/{BUILD,RPMS,SOURCES,SPECS,SRPMS}

# Create the spec file
cat > ~/rpmbuild/SPECS/reflector.spec << 'SPEC'
Name:           reflector
Version:        0.1.0
Release:        1%{?dist}
Summary:        PacketParamedic Reflector - Self-hosted network test endpoint
License:        BlueOak-1.0.0
URL:            https://github.com/packetparamedic/london-v1

%description
A self-hosted network test endpoint for PacketParamedic appliances.

%install
mkdir -p %{buildroot}/usr/local/bin
mkdir -p %{buildroot}/etc/reflector
mkdir -p %{buildroot}/var/lib/reflector
mkdir -p %{buildroot}/usr/lib/systemd/system
install -m 755 %{_sourcedir}/reflector %{buildroot}/usr/local/bin/reflector
install -m 644 %{_sourcedir}/reflector.toml %{buildroot}/etc/reflector/reflector.toml
install -m 644 %{_sourcedir}/reflector.service %{buildroot}/usr/lib/systemd/system/reflector.service

%files
/usr/local/bin/reflector
%config(noreplace) /etc/reflector/reflector.toml
/usr/lib/systemd/system/reflector.service
%dir /var/lib/reflector

%pre
getent group reflector >/dev/null || groupadd -r reflector
getent passwd reflector >/dev/null || \
    useradd -r -g reflector -d /var/lib/reflector -s /sbin/nologin reflector

%post
systemctl daemon-reload
chown reflector:reflector /var/lib/reflector

%preun
systemctl stop reflector 2>/dev/null || true
systemctl disable reflector 2>/dev/null || true
SPEC
```

### 5.2 Install the RPM

```bash
sudo dnf install ./reflector-0.1.0-1.x86_64.rpm
```

Or install manually without RPM:

```bash
# Copy the binary
sudo cp target/release/reflector /usr/local/bin/
sudo chmod 755 /usr/local/bin/reflector

# Create the system user
sudo useradd --system --no-create-home --shell /sbin/nologin reflector

# Create directories
sudo mkdir -p /etc/reflector /var/lib/reflector
sudo chown reflector:reflector /var/lib/reflector

# Copy the configuration
sudo cp /path/to/reflector.toml /etc/reflector/reflector.toml
sudo chmod 644 /etc/reflector/reflector.toml

# Create the systemd service
sudo tee /usr/lib/systemd/system/reflector.service << 'EOF'
[Unit]
Description=PacketParamedic Reflector
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=reflector
Group=reflector
ExecStart=/usr/local/bin/reflector serve
Restart=always
RestartSec=10
LimitNOFILE=65536

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/reflector
PrivateTmp=true
PrivateDevices=true
ProtectKernelTunables=true
ProtectKernelModules=true
ProtectControlGroups=true

[Install]
WantedBy=multi-user.target
EOF
```

### 5.3 Start the Service

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now reflector

# Verify
sudo systemctl status reflector
sudo journalctl -u reflector -f

# Check the Endpoint ID
sudo -u reflector reflector show-id
```

---

## 6. Bare Metal (Debian/Ubuntu)

### 6.1 Build the Deb Package

```bash
# Install build dependencies
sudo apt-get update
sudo apt-get install -y gcc make iperf3

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Build
cd reflector
cargo build --release
strip target/release/reflector

# Build the deb (if build script exists)
# ./debian/build-deb.sh

# Or package manually with dpkg-deb:
mkdir -p /tmp/reflector-deb/DEBIAN
mkdir -p /tmp/reflector-deb/usr/local/bin
mkdir -p /tmp/reflector-deb/etc/reflector
mkdir -p /tmp/reflector-deb/var/lib/reflector
mkdir -p /tmp/reflector-deb/lib/systemd/system

cp target/release/reflector /tmp/reflector-deb/usr/local/bin/
chmod 755 /tmp/reflector-deb/usr/local/bin/reflector

# Create control file
cat > /tmp/reflector-deb/DEBIAN/control << 'EOF'
Package: reflector
Version: 0.1.0
Section: net
Priority: optional
Architecture: amd64
Depends: iperf3
Maintainer: PacketParamedic <support@packetparamedic.com>
Description: PacketParamedic Reflector - Self-hosted network test endpoint
 A self-hosted network test endpoint for PacketParamedic appliances.
 Provides throughput testing (iperf3), latency testing (UDP echo),
 and system metadata collection over mutual TLS.
EOF

# Create postinst script
cat > /tmp/reflector-deb/DEBIAN/postinst << 'SCRIPT'
#!/bin/bash
set -e
getent group reflector >/dev/null || groupadd --system reflector
getent passwd reflector >/dev/null || \
    useradd --system --no-create-home --shell /usr/sbin/nologin -g reflector reflector
chown reflector:reflector /var/lib/reflector
systemctl daemon-reload
SCRIPT
chmod 755 /tmp/reflector-deb/DEBIAN/postinst

# Build the package
dpkg-deb --build /tmp/reflector-deb /tmp/reflector_0.1.0_amd64.deb
```

### 6.2 Install the Deb Package

```bash
sudo dpkg -i reflector_0.1.0_amd64.deb

# Install any missing dependencies
sudo apt-get install -f
```

Or install manually without a deb package (same process as RPM bare-metal,
adjusting paths for Debian conventions):

```bash
sudo cp target/release/reflector /usr/local/bin/
sudo useradd --system --no-create-home --shell /usr/sbin/nologin reflector
sudo mkdir -p /etc/reflector /var/lib/reflector
sudo chown reflector:reflector /var/lib/reflector

# Create systemd service (same as RPM section above)
sudo tee /lib/systemd/system/reflector.service << 'EOF'
[Unit]
Description=PacketParamedic Reflector
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=reflector
Group=reflector
ExecStart=/usr/local/bin/reflector serve
Restart=always
RestartSec=10
LimitNOFILE=65536
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/reflector
PrivateTmp=true
PrivateDevices=true

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable --now reflector
```

### 6.3 Verify

```bash
sudo systemctl status reflector
sudo -u reflector reflector show-id
```

---

## 7. Firewall Configuration

### 7.1 UFW (Ubuntu / Debian)

```bash
# Tunneled mode (default) -- only control plane port
sudo ufw allow 4000/tcp comment "Reflector mTLS control plane"

# Direct Ephemeral mode -- also open data ports
sudo ufw allow 4000/tcp comment "Reflector mTLS control plane"
sudo ufw allow 5201:5210/tcp comment "Reflector iperf3 data"
sudo ufw allow 5201:5210/udp comment "Reflector UDP echo data"

# Verify
sudo ufw status verbose
```

### 7.2 firewalld (Alma Linux / RHEL / Fedora)

```bash
# Tunneled mode
sudo firewall-cmd --permanent --add-port=4000/tcp
sudo firewall-cmd --reload

# Direct Ephemeral mode
sudo firewall-cmd --permanent --add-port=4000/tcp
sudo firewall-cmd --permanent --add-port=5201-5210/tcp
sudo firewall-cmd --permanent --add-port=5201-5210/udp
sudo firewall-cmd --reload

# Verify
sudo firewall-cmd --list-all

# Or create a dedicated service definition:
sudo tee /etc/firewalld/services/reflector.xml << 'EOF'
<?xml version="1.0" encoding="utf-8"?>
<service>
  <short>Reflector</short>
  <description>PacketParamedic Reflector - Self-hosted network test endpoint</description>
  <port protocol="tcp" port="4000"/>
  <port protocol="tcp" port="5201-5210"/>
  <port protocol="udp" port="5201-5210"/>
</service>
EOF

sudo firewall-cmd --permanent --add-service=reflector
sudo firewall-cmd --reload
```

### 7.3 iptables (Generic)

```bash
# Tunneled mode
sudo iptables -A INPUT -p tcp --dport 4000 -j ACCEPT

# Direct Ephemeral mode
sudo iptables -A INPUT -p tcp --dport 4000 -j ACCEPT
sudo iptables -A INPUT -p tcp --dport 5201:5210 -j ACCEPT
sudo iptables -A INPUT -p udp --dport 5201:5210 -j ACCEPT

# Save rules (depends on distribution)
sudo iptables-save > /etc/iptables/rules.v4    # Debian/Ubuntu
sudo service iptables save                       # RHEL/CentOS
```

### 7.4 Tunneled vs Direct Ephemeral Mode

| Mode | Ports Required | Use Case |
|---|---|---|
| **Tunneled** (default) | 4000/tcp only | Public VPS, strict firewalls. All data flows inside the mTLS tunnel. Slightly higher overhead. |
| **Direct Ephemeral** | 4000/tcp + 5201-5299/tcp + 5201-5299/udp | Trusted LANs, homelab. iperf3 runs on ephemeral ports for maximum throughput. Requires firewall rules for the data port range. |

Set the mode in `reflector.toml`:

```toml
[network]
mode = "tunneled"           # or "direct_ephemeral"
data_port_range_start = 5201
data_port_range_end = 5210  # Adjust range width as needed
```

---

## 8. Post-Installation

### 8.1 First Run and Identity Generation

On first startup, the reflector:

1. Checks for an existing identity key at the configured `private_key_path`
2. If no key exists: generates a new Ed25519 keypair and saves it
3. Derives the Endpoint ID from the public key
4. Generates a self-signed X.509 certificate embedding the Endpoint ID
5. Starts the mTLS listener

The Endpoint ID is printed at startup and can be retrieved later:

```bash
# Container deployments
docker exec reflector reflector show-id

# Bare-metal deployments
sudo -u reflector reflector show-id

# Systemd-managed (via journal)
journalctl -u reflector | grep "Endpoint ID"
```

**Important:** Back up the identity key file. If lost, the reflector will
generate a new identity on next startup, and all previously paired peers will
need to re-pair.

```bash
# Backup
sudo cp /var/lib/reflector/identity.ed25519 /safe/backup/location/
```

### 8.2 Pairing with a PacketParamedic Appliance

1. **Get the reflector's Endpoint ID:**
   ```bash
   reflector show-id
   ```

2. **Enable pairing mode:**
   ```bash
   reflector pair --ttl 10m
   ```
   Note the Endpoint ID and pairing token.

3. **On the PacketParamedic appliance:** Enter the reflector's Endpoint ID and
   pairing token in the appliance's configuration.

4. **The appliance connects to the reflector via mTLS.** The reflector verifies
   the pairing token and adds the appliance to the authorized peers list.

5. **Verify the pairing succeeded:**
   ```bash
   # Check the audit log
   cat /var/lib/reflector/audit.jsonl | jq 'select(.event_type == "peer_paired")'
   ```

For environments where interactive pairing is not practical, pre-authorize the
appliance's Endpoint ID in the configuration:

```toml
[access]
authorized_peers = [
    "PP-AAAA-BBBB-CCCC-0",  # Living room appliance
    "PP-DDDD-EEEE-FFFF-1",  # Office appliance
]
```

### 8.3 Verifying mTLS Connectivity

From the appliance or a test client that has a valid Ed25519 identity:

```bash
# Quick port check (does not verify mTLS)
nc -zv <reflector-host> 4000

# TLS check (will show cert info but fail without client cert)
echo | openssl s_client -connect <reflector-host>:4000 -brief 2>&1

# Full mTLS check requires a properly configured client with:
# - An Ed25519 certificate containing the client's Endpoint ID
# - TLS 1.3
# - ALPN: pp-link/1
```

### 8.4 Monitoring and Alerting

#### Health Check

The reflector includes a `status` command that can be used as a health check:

```bash
reflector status
```

Exit code 0 indicates the reflector is healthy (has a valid identity).

For container deployments, the Containerfile includes a `HEALTHCHECK` directive:

```dockerfile
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s \
    CMD ["/usr/local/bin/reflector", "status"]
```

#### Log Monitoring

Monitor the audit log for security events:

```bash
# Watch for denied connections (potential intrusion attempts)
tail -f /var/lib/reflector/audit.jsonl | \
  jq -r 'select(.event_type == "connection_denied") | "\(.timestamp) DENIED \(.peer_id) \(.reason)"'

# Watch for all events
tail -f /var/lib/reflector/audit.jsonl | jq -c .
```

#### Metrics (Future)

A Prometheus-compatible `/metrics` endpoint is planned for a future release.
In the meantime, the `get_status` and `get_path_meta` protocol messages
provide runtime metrics:

- Uptime
- Active test count
- Tests completed today
- Bytes transferred today
- CPU load, memory usage, load averages
- MTU and NTP sync status

#### Alerting Recommendations

| Condition | Severity | Action |
|---|---|---|
| Reflector process not running | Critical | Restart via systemd |
| Identity key file missing | Critical | Check backup, investigate |
| Audit log write failure | High | Check disk space and permissions |
| Repeated `connection_denied` from same IP | Medium | Investigate potential scan |
| `quota_exceeded` for legitimate peer | Low | Increase daily byte quota |
| High CPU during idle | Low | Check for stuck sessions |

### 8.5 Log Rotation

The audit log is append-only and will grow over time. Set up log rotation:

```bash
# Using logrotate (create /etc/logrotate.d/reflector)
sudo tee /etc/logrotate.d/reflector << 'EOF'
/var/lib/reflector/audit.jsonl {
    daily
    rotate 30
    compress
    missingok
    notifempty
    copytruncate
}
EOF
```

**Note:** `copytruncate` is used because the reflector keeps the file open.
This avoids the need to send a signal to the reflector process.

### 8.6 Backup Strategy

| Item | Location | Backup Frequency | Recovery |
|---|---|---|---|
| Identity key | `/var/lib/reflector/identity.ed25519` | Once (after first run) | Restore file; same Endpoint ID |
| Configuration | `/etc/reflector/reflector.toml` | After changes | Restore file; restart reflector |
| Audit log | `/var/lib/reflector/audit.jsonl` | Daily (optional) | For compliance/forensics only |

If the identity key is lost, the reflector generates a new one on next startup.
This changes the Endpoint ID, requiring all peers to re-pair.
