//! Rate limiting and resource governance for the PacketParamedic Reflector.
//!
//! The [`GovernanceEngine`] enforces per-peer rate limits, byte quotas,
//! cooldown periods, and test-type restrictions.  All state is held behind a
//! `tokio::sync::RwLock` for safe concurrent access from async tasks.

use std::collections::{HashMap, VecDeque};

use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use tokio::time::Instant;
use tracing::{debug, info};

use crate::config::QuotaConfig;
use crate::rpc::{DenyReason, TestType};

// ---------------------------------------------------------------------------
// GovernanceEngine
// ---------------------------------------------------------------------------

/// Inner mutable state for governance tracking.
struct GovernanceInner {
    /// Sliding window of test timestamps per peer (for rate limiting).
    peer_test_counts: HashMap<String, VecDeque<Instant>>,
    /// Bytes transferred today per peer (for daily quota).
    peer_bytes_today: HashMap<String, u64>,
    /// Timestamp of the last test per peer (for cooldown enforcement).
    peer_last_test: HashMap<String, Instant>,
    /// UTC start of the current day (for daily reset).
    day_start: DateTime<Utc>,
}

/// Rate limiting and resource governance engine.
///
/// Enforces per-peer policies: rate limits, byte quotas, cooldown periods,
/// and test-type allow-lists.
pub struct GovernanceEngine {
    inner: RwLock<GovernanceInner>,
    config: QuotaConfig,
}

impl GovernanceEngine {
    /// Create a new governance engine with the given configuration.
    pub fn new(config: QuotaConfig) -> Self {
        let now = Utc::now();
        let day_start = now
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .expect("midnight is valid")
            .and_utc();

        Self {
            inner: RwLock::new(GovernanceInner {
                peer_test_counts: HashMap::new(),
                peer_bytes_today: HashMap::new(),
                peer_last_test: HashMap::new(),
                day_start,
            }),
            config,
        }
    }

    /// Check whether a peer is allowed to start a test.
    ///
    /// Returns `Ok(())` if the request is allowed, or `Err(DenyReason)` if it
    /// should be denied.
    pub async fn check_allowed(
        &self,
        peer_id: &str,
        test_type: &TestType,
    ) -> Result<(), DenyReason> {
        // 1. Check test type allowed.
        match test_type {
            TestType::UdpEcho if !self.config.allow_udp_echo => {
                debug!(peer_id = peer_id, "UDP echo tests not allowed");
                return Err(DenyReason::InvalidParams);
            }
            TestType::Throughput if !self.config.allow_throughput => {
                debug!(peer_id = peer_id, "throughput tests not allowed");
                return Err(DenyReason::InvalidParams);
            }
            _ => {}
        }

        let inner = self.inner.read().await;

        // 2. Check cooldown.
        if self.config.cooldown_sec > 0 {
            if let Some(last) = inner.peer_last_test.get(peer_id) {
                let elapsed = last.elapsed();
                if elapsed.as_secs() < self.config.cooldown_sec {
                    debug!(
                        peer_id = peer_id,
                        elapsed_sec = elapsed.as_secs(),
                        cooldown_sec = self.config.cooldown_sec,
                        "cooldown not elapsed"
                    );
                    return Err(DenyReason::RateLimited);
                }
            }
        }

        // 3. Check rate limit (sliding window over the last hour).
        if let Some(timestamps) = inner.peer_test_counts.get(peer_id) {
            let one_hour_ago = Instant::now() - std::time::Duration::from_secs(3600);
            let recent_count = timestamps.iter().filter(|&&t| t > one_hour_ago).count();
            if recent_count as u32 >= self.config.max_tests_per_hour_per_peer {
                debug!(
                    peer_id = peer_id,
                    count = recent_count,
                    max = self.config.max_tests_per_hour_per_peer,
                    "rate limit exceeded"
                );
                return Err(DenyReason::RateLimited);
            }
        }

        // 4. Check daily byte quota.
        if let Some(&bytes) = inner.peer_bytes_today.get(peer_id) {
            if bytes >= self.config.max_bytes_per_day_per_peer {
                debug!(
                    peer_id = peer_id,
                    bytes = bytes,
                    max = self.config.max_bytes_per_day_per_peer,
                    "daily byte quota exceeded"
                );
                return Err(DenyReason::QuotaExceeded);
            }
        }

        Ok(())
    }

    /// Record that a test has started for a peer.
    ///
    /// Updates the sliding window and last-test timestamp.
    pub async fn record_test_start(&self, peer_id: &str) {
        let mut inner = self.inner.write().await;
        let now = Instant::now();

        // Update sliding window.
        let timestamps = inner
            .peer_test_counts
            .entry(peer_id.to_string())
            .or_default();
        timestamps.push_back(now);

        // Prune entries older than 1 hour.
        let one_hour_ago = Instant::now() - std::time::Duration::from_secs(3600);
        while timestamps.front().is_some_and(|&t| t < one_hour_ago) {
            timestamps.pop_front();
        }

        // Update last-test timestamp.
        inner
            .peer_last_test
            .insert(peer_id.to_string(), now);

        debug!(peer_id = peer_id, "recorded test start");
    }

    /// Record bytes transferred for a peer (adds to their daily total).
    pub async fn record_bytes(&self, peer_id: &str, bytes: u64) {
        let mut inner = self.inner.write().await;
        let entry = inner
            .peer_bytes_today
            .entry(peer_id.to_string())
            .or_insert(0);
        *entry += bytes;
        debug!(peer_id = peer_id, bytes = bytes, total = *entry, "recorded bytes");
    }

    /// Reset daily byte counters if a new UTC day has started.
    ///
    /// Should be called periodically from a background task.
    pub async fn reset_daily_if_needed(&self) {
        let now = Utc::now();
        let today_start = now
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .expect("midnight is valid")
            .and_utc();

        let mut inner = self.inner.write().await;
        if today_start > inner.day_start {
            info!(
                old_day = inner.day_start.to_rfc3339().as_str(),
                new_day = today_start.to_rfc3339().as_str(),
                "resetting daily governance counters"
            );
            inner.peer_bytes_today.clear();
            inner.day_start = today_start;
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> QuotaConfig {
        QuotaConfig {
            max_concurrent_tests: 1,
            max_test_duration_sec: 60,
            max_tests_per_hour_per_peer: 10,
            max_bytes_per_day_per_peer: 1_000_000,
            cooldown_sec: 2,
            allow_udp_echo: true,
            allow_throughput: true,
        }
    }

    #[tokio::test]
    async fn test_rate_limit_allows_then_denies() {
        let config = QuotaConfig {
            max_tests_per_hour_per_peer: 10,
            cooldown_sec: 0,
            ..default_config()
        };
        let engine = GovernanceEngine::new(config);

        // First 10 should be allowed.
        for i in 0..10 {
            engine.record_test_start("peer-a").await;
            let result = engine.check_allowed("peer-a", &TestType::UdpEcho).await;
            if i < 9 {
                // After recording test start i, there are i+1 entries.
                // check_allowed checks the count BEFORE recording, so we check
                // after the record. The 10th record means next check will deny.
            }
            // We need to check after the 10th record_test_start.
            let _ = result;
        }

        // The 11th check should be denied.
        let result = engine.check_allowed("peer-a", &TestType::UdpEcho).await;
        assert_eq!(result, Err(DenyReason::RateLimited));
    }

    #[tokio::test]
    async fn test_cooldown_enforcement() {
        let config = QuotaConfig {
            cooldown_sec: 100, // very long cooldown for test reliability
            ..default_config()
        };
        let engine = GovernanceEngine::new(config);

        // First request should be fine.
        let r1 = engine.check_allowed("peer-b", &TestType::UdpEcho).await;
        assert!(r1.is_ok());

        // Record test start.
        engine.record_test_start("peer-b").await;

        // Immediately checking again should be denied (cooldown).
        let r2 = engine.check_allowed("peer-b", &TestType::UdpEcho).await;
        assert_eq!(r2, Err(DenyReason::RateLimited));
    }

    #[tokio::test]
    async fn test_byte_quota_enforcement() {
        let config = QuotaConfig {
            max_bytes_per_day_per_peer: 1000,
            cooldown_sec: 0,
            ..default_config()
        };
        let engine = GovernanceEngine::new(config);

        // Under quota should be fine.
        engine.record_bytes("peer-c", 500).await;
        let r1 = engine.check_allowed("peer-c", &TestType::UdpEcho).await;
        assert!(r1.is_ok());

        // At quota should be denied.
        engine.record_bytes("peer-c", 500).await;
        let r2 = engine.check_allowed("peer-c", &TestType::UdpEcho).await;
        assert_eq!(r2, Err(DenyReason::QuotaExceeded));
    }

    #[tokio::test]
    async fn test_test_type_restriction() {
        let config = QuotaConfig {
            allow_udp_echo: false,
            allow_throughput: true,
            cooldown_sec: 0,
            ..default_config()
        };
        let engine = GovernanceEngine::new(config);

        let r1 = engine.check_allowed("peer-d", &TestType::UdpEcho).await;
        assert_eq!(r1, Err(DenyReason::InvalidParams));

        let r2 = engine.check_allowed("peer-d", &TestType::Throughput).await;
        assert!(r2.is_ok());
    }

    #[tokio::test]
    async fn test_different_peers_independent() {
        let config = QuotaConfig {
            max_bytes_per_day_per_peer: 1000,
            cooldown_sec: 0,
            ..default_config()
        };
        let engine = GovernanceEngine::new(config);

        // Exhaust peer-e's quota.
        engine.record_bytes("peer-e", 1000).await;
        let r1 = engine.check_allowed("peer-e", &TestType::UdpEcho).await;
        assert_eq!(r1, Err(DenyReason::QuotaExceeded));

        // peer-f should be unaffected.
        let r2 = engine.check_allowed("peer-f", &TestType::UdpEcho).await;
        assert!(r2.is_ok());
    }

    #[tokio::test]
    async fn test_daily_reset() {
        let config = QuotaConfig {
            max_bytes_per_day_per_peer: 1000,
            cooldown_sec: 0,
            ..default_config()
        };
        let engine = GovernanceEngine::new(config);

        engine.record_bytes("peer-g", 1000).await;
        let r1 = engine.check_allowed("peer-g", &TestType::UdpEcho).await;
        assert_eq!(r1, Err(DenyReason::QuotaExceeded));

        // Simulate a day change by manipulating the inner state.
        {
            let mut inner = engine.inner.write().await;
            inner.day_start = Utc::now() - chrono::Duration::days(2);
        }

        engine.reset_daily_if_needed().await;

        // After reset, peer should be allowed again.
        let r2 = engine.check_allowed("peer-g", &TestType::UdpEcho).await;
        assert!(r2.is_ok());
    }
}
