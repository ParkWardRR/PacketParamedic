use std::time::Duration;
use serde::{Deserialize, Serialize};
use anyhow::Result;

pub mod ookla;
pub mod ndt7;
pub mod fast;

/// Metadata describing a speed test provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderMeta {
    pub id: &'static str,              // "ookla-cli", "ndt7", "fast-cli", "iperf3"
    pub display_name: &'static str,     // "Speedtest.net (Ookla)", "NDT (M-Lab)", ...
    pub kind: ProviderKind,             // PublicWAN, SelfHostedWAN, SelfHostedLAN, BrowserAutomated
    pub recommendation: Recommendation, // Recommended, Optional, Fallback
    pub description: &'static str,      // "Most recognizable benchmark; broad server network..."
    pub install_hint: &'static str,     // "Install official CLI via apt/brew"
    pub licensing_note: Option<&'static str>,   // "Personal use only on single device (EULA)"
    pub stability: Stability,           // Stable, Beta, Experimental
    pub metrics: MetricsSupported,      // dl/ul/latency/jitter/loss/bufferbloat
}

/// Helper enum for UI sorting/presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Recommendation {
    Recommended,
    Optional,
    Fallback, // e.g. Playwright
}

/// The type of provider, determining what it tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderKind { 
    PublicWAN, 
    SelfHostedWAN, 
    SelfHostedLAN, 
    BrowserAutomated 
}

/// Stability level of the provider implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stability { 
    Stable, 
    Beta, 
    Experimental 
}

/// Which metrics this provider is capable of returning.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct MetricsSupported {
    pub download: bool,
    pub upload: bool,
    pub latency: bool,
    pub jitter: bool,
    pub packet_loss: bool,
    pub bufferbloat: bool,
}

/// Configuration for a specific test run.
#[derive(Debug, Clone)]
pub struct SpeedTestRequest {
    pub timeout: Duration,
    pub prefer_ipv6: bool,
    pub server_hint: Option<String>, // provider-specific: server id, fqdn, region, etc.
}

/// Normalized result from any provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeedTestResult {
    pub provider_id: String,
    pub download_mbps: Option<f64>,
    pub upload_mbps: Option<f64>,
    pub latency_ms: Option<f64>,
    pub jitter_ms: Option<f64>,
    pub packet_loss_pct: Option<f64>,
    pub bufferbloat_ms: Option<f64>,
    pub raw_json: Option<serde_json::Value>, // keep provider-native detail
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Trait for all speed test providers (Ookla, NDT7, Fast, iPerf3).
pub trait SpeedTestProvider: Send + Sync {
    /// Static metadata about the provider.
    fn meta(&self) -> ProviderMeta;
    
    /// Check if the provider's CLI/dependency is available.
    fn is_available(&self) -> bool; 
    
    /// Run the speed test.
    fn run(&self, req: SpeedTestRequest) -> Result<SpeedTestResult>;
}
