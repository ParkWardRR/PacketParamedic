//! TOML configuration for the PacketParamedic Reflector.
//!
//! Implements Section 12 of the spec: a layered configuration model with
//! sensible defaults, environment variable override for the config file path,
//! and standard filesystem locations.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

// ---------------------------------------------------------------------------
// Top-level config
// ---------------------------------------------------------------------------

/// Root configuration for the reflector process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectorConfig {
    #[serde(default)]
    pub identity: IdentityConfig,
    #[serde(default)]
    pub network: NetworkConfig,
    #[serde(default)]
    pub access: AccessConfig,
    #[serde(default)]
    pub quotas: QuotaConfig,
    #[serde(default)]
    pub iperf3: Iperf3Config,
    #[serde(default)]
    pub logging: LoggingConfig,
}

impl Default for ReflectorConfig {
    fn default() -> Self {
        Self {
            identity: IdentityConfig::default(),
            network: NetworkConfig::default(),
            access: AccessConfig::default(),
            quotas: QuotaConfig::default(),
            iperf3: Iperf3Config::default(),
            logging: LoggingConfig::default(),
        }
    }
}

impl ReflectorConfig {
    /// Load configuration from a TOML file at `path`.
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read config file: {}", path.display()))?;
        let config: Self = toml::from_str(&content)
            .with_context(|| format!("failed to parse config file: {}", path.display()))?;
        info!(path = %path.display(), "loaded reflector configuration");
        Ok(config)
    }

    /// Try to load configuration from, in order:
    /// 1. The path specified by the `REFLECTOR_CONFIG` environment variable.
    /// 2. `/etc/reflector/reflector.toml`.
    /// 3. Fall back to compiled-in defaults.
    pub fn load_or_default() -> Self {
        // 1. Environment variable override.
        if let Ok(env_path) = std::env::var("REFLECTOR_CONFIG") {
            let path = Path::new(&env_path);
            match Self::load(path) {
                Ok(cfg) => return cfg,
                Err(e) => {
                    warn!(
                        path = %path.display(),
                        error = %e,
                        "REFLECTOR_CONFIG set but file could not be loaded, trying fallback"
                    );
                }
            }
        }

        // 2. Standard system location.
        let system_path = Path::new("/etc/reflector/reflector.toml");
        if system_path.exists() {
            match Self::load(system_path) {
                Ok(cfg) => return cfg,
                Err(e) => {
                    warn!(
                        path = %system_path.display(),
                        error = %e,
                        "system config file exists but could not be loaded, using defaults"
                    );
                }
            }
        }

        // 3. Defaults.
        debug!("no config file found, using compiled-in defaults");
        Self::default()
    }
}

// ---------------------------------------------------------------------------
// Identity
// ---------------------------------------------------------------------------

/// Identity key storage configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct IdentityConfig {
    /// Path to the Ed25519 private key file (raw 32-byte secret).
    pub private_key_path: PathBuf,
}

impl Default for IdentityConfig {
    fn default() -> Self {
        Self {
            private_key_path: PathBuf::from("/var/lib/reflector/identity.ed25519"),
        }
    }
}

// ---------------------------------------------------------------------------
// Network
// ---------------------------------------------------------------------------

/// Network listener and data-plane configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NetworkConfig {
    /// Address and port for the QUIC / TLS control plane listener.
    pub listen_address: String,
    /// ALPN protocol identifier negotiated during the TLS handshake.
    pub alpn: String,
    /// Data-plane transport mode.
    pub mode: DataPlaneMode,
    /// Start of the port range reserved for iperf3 / data-plane sockets.
    pub data_port_range_start: u16,
    /// End of the port range (inclusive).
    pub data_port_range_end: u16,
    /// Network deployment mode: `"auto"` (default), `"wan"`, `"lan"`, or `"hybrid"`.
    /// Controls how the reflector reports its network position to peers.
    pub deployment_mode: String,
    /// Address and port for the HTTP health check listener.
    pub listen_address_health: String,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            listen_address: "0.0.0.0:4000".to_string(),
            alpn: "pp-link/1".to_string(),
            mode: DataPlaneMode::Tunneled,
            data_port_range_start: 5201,
            data_port_range_end: 5299,
            deployment_mode: "auto".to_string(),
            listen_address_health: "0.0.0.0:7301".to_string(),
        }
    }
}

/// How the data plane (iperf3 / UDP echo) traffic is transported.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataPlaneMode {
    /// All data flows inside the mTLS tunnel (default, firewall-friendly).
    Tunneled,
    /// iperf3 opens ephemeral ports directly (lower overhead, requires firewall
    /// rules for the configured port range).
    DirectEphemeral,
}

// ---------------------------------------------------------------------------
// Access
// ---------------------------------------------------------------------------

/// Peer authorization and pairing configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AccessConfig {
    /// Whether the pairing endpoint is enabled (allows new peers to enroll).
    pub pairing_enabled: bool,
    /// List of pre-authorized peer endpoint IDs (e.g. `PP-XXXX-...`).
    pub authorized_peers: Vec<String>,
}

impl Default for AccessConfig {
    fn default() -> Self {
        Self {
            pairing_enabled: false,
            authorized_peers: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Quotas
// ---------------------------------------------------------------------------

/// Resource quota / rate-limit configuration per-peer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct QuotaConfig {
    /// Maximum duration for a single test session (seconds).
    pub max_test_duration_sec: u64,
    /// Maximum number of tests running concurrently on this reflector.
    pub max_concurrent_tests: u32,
    /// Per-peer rate limit: tests allowed per rolling hour.
    pub max_tests_per_hour_per_peer: u32,
    /// Per-peer daily transfer cap (bytes).  Default: 5 GB.
    pub max_bytes_per_day_per_peer: u64,
    /// Minimum cooldown between consecutive tests from the same peer (seconds).
    pub cooldown_sec: u64,
    /// Whether UDP echo (latency / jitter) tests are permitted.
    pub allow_udp_echo: bool,
    /// Whether throughput (iperf3) tests are permitted.
    pub allow_throughput: bool,
}

impl Default for QuotaConfig {
    fn default() -> Self {
        Self {
            max_test_duration_sec: 60,
            max_concurrent_tests: 1,
            max_tests_per_hour_per_peer: 10,
            max_bytes_per_day_per_peer: 5_000_000_000,
            cooldown_sec: 5,
            allow_udp_echo: true,
            allow_throughput: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Iperf3
// ---------------------------------------------------------------------------

/// Configuration for the iperf3 subprocess used by throughput tests.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Iperf3Config {
    /// Path (or bare command name resolved via `$PATH`) to the iperf3 binary.
    pub path: String,
    /// Default number of parallel streams per test.
    pub default_streams: u32,
    /// Hard upper bound on parallel streams a peer may request.
    pub max_streams: u32,
}

impl Default for Iperf3Config {
    fn default() -> Self {
        Self {
            path: "iperf3".to_string(),
            default_streams: 4,
            max_streams: 8,
        }
    }
}

// ---------------------------------------------------------------------------
// Logging
// ---------------------------------------------------------------------------

/// Logging and audit trail configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    /// Minimum tracing level (`trace`, `debug`, `info`, `warn`, `error`).
    pub level: String,
    /// Path to the append-only JSON-lines audit log.
    pub audit_log_path: PathBuf,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            audit_log_path: PathBuf::from("/var/lib/reflector/audit.jsonl"),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults_are_sane() {
        let cfg = ReflectorConfig::default();

        // Identity
        assert_eq!(
            cfg.identity.private_key_path,
            PathBuf::from("/var/lib/reflector/identity.ed25519")
        );

        // Network
        assert_eq!(cfg.network.listen_address, "0.0.0.0:4000");
        assert_eq!(cfg.network.alpn, "pp-link/1");
        assert!(matches!(cfg.network.mode, DataPlaneMode::Tunneled));
        assert_eq!(cfg.network.data_port_range_start, 5201);
        assert_eq!(cfg.network.data_port_range_end, 5299);
        assert_eq!(cfg.network.listen_address_health, "0.0.0.0:7301"); // Default check

        // Access
        assert!(!cfg.access.pairing_enabled);
        assert!(cfg.access.authorized_peers.is_empty());

        // Quotas
        assert_eq!(cfg.quotas.max_test_duration_sec, 60);
        assert_eq!(cfg.quotas.max_concurrent_tests, 1);
        assert_eq!(cfg.quotas.max_tests_per_hour_per_peer, 10);
        assert_eq!(cfg.quotas.max_bytes_per_day_per_peer, 5_000_000_000);
        assert_eq!(cfg.quotas.cooldown_sec, 5);
        assert!(cfg.quotas.allow_udp_echo);
        assert!(cfg.quotas.allow_throughput);

        // Iperf3
        assert_eq!(cfg.iperf3.path, "iperf3");
        assert_eq!(cfg.iperf3.default_streams, 4);
        assert_eq!(cfg.iperf3.max_streams, 8);

        // Logging
        assert_eq!(cfg.logging.level, "info");
        assert_eq!(
            cfg.logging.audit_log_path,
            PathBuf::from("/var/lib/reflector/audit.jsonl")
        );
    }

    #[test]
    fn test_parse_example_toml() {
        let toml_str = r#"
[identity]
private_key_path = "/opt/reflector/my.key"

[network]
listen_address = "127.0.0.1:5000"
alpn = "pp-link/1"
mode = "direct_ephemeral"
data_port_range_start = 6000
data_port_range_end = 6100

[access]
pairing_enabled = true
authorized_peers = ["PP-AAAA-BBBB-CCCC-0", "PP-DDDD-EEEE-FFFF-1"]

[quotas]
max_test_duration_sec = 120
max_concurrent_tests = 4
max_tests_per_hour_per_peer = 20
max_bytes_per_day_per_peer = 10000000000
cooldown_sec = 10
allow_udp_echo = false
allow_throughput = true

[iperf3]
path = "/usr/local/bin/iperf3"
default_streams = 2
max_streams = 16

[logging]
level = "debug"
audit_log_path = "/var/log/reflector/audit.jsonl"
"#;

        let cfg: ReflectorConfig = toml::from_str(toml_str).unwrap();

        assert_eq!(cfg.identity.private_key_path, PathBuf::from("/opt/reflector/my.key"));
        assert_eq!(cfg.network.listen_address, "127.0.0.1:5000");
        assert!(matches!(cfg.network.mode, DataPlaneMode::DirectEphemeral));
        assert_eq!(cfg.network.data_port_range_start, 6000);
        assert_eq!(cfg.network.data_port_range_end, 6100);
        assert!(cfg.access.pairing_enabled);
        assert_eq!(cfg.access.authorized_peers.len(), 2);
        assert_eq!(cfg.access.authorized_peers[0], "PP-AAAA-BBBB-CCCC-0");
        assert_eq!(cfg.quotas.max_test_duration_sec, 120);
        assert_eq!(cfg.quotas.max_concurrent_tests, 4);
        assert_eq!(cfg.quotas.max_tests_per_hour_per_peer, 20);
        assert_eq!(cfg.quotas.max_bytes_per_day_per_peer, 10_000_000_000);
        assert_eq!(cfg.quotas.cooldown_sec, 10);
        assert!(!cfg.quotas.allow_udp_echo);
        assert!(cfg.quotas.allow_throughput);
        assert_eq!(cfg.iperf3.path, "/usr/local/bin/iperf3");
        assert_eq!(cfg.iperf3.default_streams, 2);
        assert_eq!(cfg.iperf3.max_streams, 16);
        assert_eq!(cfg.logging.level, "debug");
        assert_eq!(
            cfg.logging.audit_log_path,
            PathBuf::from("/var/log/reflector/audit.jsonl")
        );
    }

    #[test]
    fn test_partial_toml_uses_defaults() {
        let toml_str = r#"
[network]
listen_address = "10.0.0.1:8080"
"#;

        let cfg: ReflectorConfig = toml::from_str(toml_str).unwrap();

        // Explicit override.
        assert_eq!(cfg.network.listen_address, "10.0.0.1:8080");

        // Everything else should be defaults.
        assert_eq!(
            cfg.identity.private_key_path,
            PathBuf::from("/var/lib/reflector/identity.ed25519")
        );
        assert!(!cfg.access.pairing_enabled);
        assert_eq!(cfg.quotas.max_test_duration_sec, 60);
        assert_eq!(cfg.iperf3.path, "iperf3");
        assert_eq!(cfg.logging.level, "info");
    }

    #[test]
    fn test_empty_toml_uses_all_defaults() {
        let cfg: ReflectorConfig = toml::from_str("").unwrap();
        let defaults = ReflectorConfig::default();

        assert_eq!(cfg.network.listen_address, defaults.network.listen_address);
        assert_eq!(
            cfg.identity.private_key_path,
            defaults.identity.private_key_path
        );
        assert_eq!(
            cfg.quotas.max_bytes_per_day_per_peer,
            defaults.quotas.max_bytes_per_day_per_peer
        );
    }

    #[test]
    fn test_load_from_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("reflector.toml");
        std::fs::write(
            &path,
            r#"
[network]
listen_address = "0.0.0.0:9999"
"#,
        )
        .unwrap();

        let cfg = ReflectorConfig::load(&path).unwrap();
        assert_eq!(cfg.network.listen_address, "0.0.0.0:9999");
    }

    #[test]
    fn test_load_missing_file_errors() {
        let result = ReflectorConfig::load(Path::new("/nonexistent/path/reflector.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let cfg = ReflectorConfig::default();
        let toml_str = toml::to_string_pretty(&cfg).unwrap();
        let roundtripped: ReflectorConfig = toml::from_str(&toml_str).unwrap();

        assert_eq!(
            cfg.network.listen_address,
            roundtripped.network.listen_address
        );
        assert_eq!(
            cfg.quotas.max_bytes_per_day_per_peer,
            roundtripped.quotas.max_bytes_per_day_per_peer
        );
        assert_eq!(cfg.iperf3.max_streams, roundtripped.iperf3.max_streams);
    }
}
