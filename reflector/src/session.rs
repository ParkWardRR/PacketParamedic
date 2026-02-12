//! Session manager for the PacketParamedic Reflector.
//!
//! Tracks active test sessions, enforces concurrency limits, delegates
//! governance checks, and provides session lifecycle management.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::config::QuotaConfig;
use crate::governance::GovernanceEngine;
use crate::rpc::{
    ActiveTestInfo, DenyReason, SessionDeny, SessionGrant, StatusSnapshot, TestParams, TestType,
};

// ---------------------------------------------------------------------------
// ActiveSession
// ---------------------------------------------------------------------------

/// A currently running test session.
pub struct ActiveSession {
    /// Unique test identifier (UUID).
    pub test_id: String,
    /// Kind of test being run.
    pub test_type: TestType,
    /// Identity of the remote peer.
    pub peer_id: String,
    /// Data-plane port assigned to this session.
    pub port: u16,
    /// When the test started.
    pub started_at: DateTime<Utc>,
    /// When the test grant expires.
    pub expires_at: DateTime<Utc>,
    /// Total bytes transferred so far (atomically updated).
    pub bytes_transferred: AtomicU64,
    /// PID of the child process (e.g. iperf3), if applicable.
    pub child_pid: Option<u32>,
}

impl ActiveSession {
    /// Returns the remaining seconds until expiry, clamped to zero.
    pub fn remaining_sec(&self) -> u64 {
        let remaining = self.expires_at - Utc::now();
        remaining.num_seconds().max(0) as u64
    }

    /// Convert to the wire-format `ActiveTestInfo`.
    pub fn to_info(&self) -> ActiveTestInfo {
        ActiveTestInfo {
            test_id: self.test_id.clone(),
            test_type: self.test_type.clone(),
            peer_id: self.peer_id.clone(),
            started_at: self.started_at.to_rfc3339(),
            remaining_sec: self.remaining_sec(),
        }
    }
}

// ---------------------------------------------------------------------------
// SessionManager
// ---------------------------------------------------------------------------

/// Manages the lifecycle of active test sessions.
///
/// Thread-safe via `Arc<RwLock<...>>` for use across async tasks.
pub struct SessionManager {
    /// Active sessions keyed by test_id.
    sessions: Arc<RwLock<HashMap<String, ActiveSession>>>,
    /// Governance engine for rate limiting and quota enforcement.
    governance: Arc<GovernanceEngine>,
    /// Configuration snapshot.
    config: QuotaConfig,
    /// Process start time for uptime calculation.
    started_at: DateTime<Utc>,
    /// Reflector endpoint ID (for status snapshots).
    endpoint_id: String,
}

impl SessionManager {
    /// Create a new session manager with the given quota configuration.
    pub fn new(
        config: QuotaConfig,
        governance: Arc<GovernanceEngine>,
        endpoint_id: String,
    ) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            governance,
            config,
            started_at: Utc::now(),
            endpoint_id,
        }
    }

    /// Request a new test session for a peer.
    ///
    /// Checks concurrency limits and governance rules. On success, returns a
    /// `SessionGrant` with the assigned port, token, and expiry. On failure,
    /// returns a `SessionDeny` with the reason.
    pub async fn request_session(
        &self,
        peer_id: &str,
        test_type: TestType,
        params: &TestParams,
    ) -> Result<SessionGrant, SessionDeny> {
        // 1. Check max concurrent sessions.
        {
            let sessions = self.sessions.read().await;
            if sessions.len() as u32 >= self.config.max_concurrent_tests {
                info!(
                    peer_id = peer_id,
                    active = sessions.len(),
                    max = self.config.max_concurrent_tests,
                    "session denied: server busy"
                );
                return Err(SessionDeny {
                    reason: DenyReason::Busy,
                    message: format!(
                        "server is at maximum capacity ({} concurrent tests)",
                        self.config.max_concurrent_tests
                    ),
                    retry_after_sec: Some(10),
                });
            }
        }

        // 2. Check governance rules (rate limit, cooldown, quota, test type).
        if let Err(reason) = self.governance.check_allowed(peer_id, &test_type).await {
            let (message, retry_after) = match &reason {
                DenyReason::RateLimited => (
                    "rate limit exceeded for this peer".to_string(),
                    Some(60),
                ),
                DenyReason::QuotaExceeded => (
                    "daily byte quota exceeded for this peer".to_string(),
                    None,
                ),
                DenyReason::InvalidParams => (
                    format!("test type {:?} is not allowed on this reflector", test_type),
                    None,
                ),
                _ => ("request denied by governance policy".to_string(), Some(30)),
            };
            info!(peer_id = peer_id, reason = ?reason, "session denied by governance");
            return Err(SessionDeny {
                reason,
                message,
                retry_after_sec: retry_after,
            });
        }

        // 3. Clamp duration to max.
        let duration_sec = params
            .duration_sec
            .min(self.config.max_test_duration_sec);

        // 4. Generate session identifiers.
        let test_id = Uuid::new_v4().to_string();
        let token = Uuid::new_v4().to_string();
        let now = Utc::now();
        let expires_at = now
            + Duration::seconds(duration_sec as i64)
            + Duration::seconds(5); // grace period

        // 5. Assign a port. For now use a placeholder; the engine will provide
        //    the actual port during start-up.
        let port = 0_u16; // will be updated by the engine layer

        // 6. Record the test start in governance.
        self.governance.record_test_start(peer_id).await;

        // 7. Insert the active session.
        let session = ActiveSession {
            test_id: test_id.clone(),
            test_type: test_type.clone(),
            peer_id: peer_id.to_string(),
            port,
            started_at: now,
            expires_at,
            bytes_transferred: AtomicU64::new(0),
            child_pid: None,
        };

        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(test_id.clone(), session);
        }

        debug!(
            test_id = test_id.as_str(),
            peer_id = peer_id,
            test_type = ?test_type,
            duration_sec = duration_sec,
            "session granted"
        );

        Ok(SessionGrant {
            test_id,
            mode: "direct_ephemeral".to_string(),
            port,
            token,
            expires_at: expires_at.to_rfc3339(),
        })
    }

    /// Close and remove an active session.
    pub async fn close_session(&self, test_id: &str) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.remove(test_id) {
            let bytes = session.bytes_transferred.load(Ordering::Relaxed);
            info!(
                test_id = test_id,
                peer_id = session.peer_id.as_str(),
                bytes_transferred = bytes,
                "session closed"
            );
            // Record final bytes in governance.
            self.governance
                .record_bytes(&session.peer_id, bytes)
                .await;
        } else {
            warn!(test_id = test_id, "attempted to close unknown session");
        }
        Ok(())
    }

    /// Get a snapshot of the reflector's current status.
    pub async fn get_status(&self) -> StatusSnapshot {
        let sessions = self.sessions.read().await;
        let uptime = (Utc::now() - self.started_at).num_seconds().max(0) as u64;

        let active_test = sessions.values().next().map(|s| s.to_info());

        let bytes_today: u64 = sessions
            .values()
            .map(|s| s.bytes_transferred.load(Ordering::Relaxed))
            .sum();

        StatusSnapshot {
            endpoint_id: self.endpoint_id.clone(),
            uptime_sec: uptime,
            active_test,
            tests_today: 0, // TODO: track completed tests count
            bytes_today,
            network_position: None, // populated by server layer
        }
    }

    /// Remove all sessions that have exceeded their expiry time.
    ///
    /// Should be called periodically from a background task.
    pub async fn cleanup_expired(&self) {
        let now = Utc::now();
        let mut sessions = self.sessions.write().await;
        let expired: Vec<String> = sessions
            .iter()
            .filter(|(_, s)| s.expires_at < now)
            .map(|(id, _)| id.clone())
            .collect();

        for test_id in &expired {
            if let Some(session) = sessions.remove(test_id) {
                let bytes = session.bytes_transferred.load(Ordering::Relaxed);
                warn!(
                    test_id = test_id.as_str(),
                    peer_id = session.peer_id.as_str(),
                    bytes_transferred = bytes,
                    "expired session cleaned up"
                );
            }
        }

        if !expired.is_empty() {
            info!(count = expired.len(), "cleaned up expired sessions");
        }
    }

    /// Record bytes transferred for an active session.
    pub async fn record_bytes(&self, test_id: &str, bytes: u64) {
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(test_id) {
            session
                .bytes_transferred
                .fetch_add(bytes, Ordering::Relaxed);
        }
    }

    /// Update the port for an active session (called after engine starts).
    pub async fn set_session_port(&self, test_id: &str, port: u16) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(test_id) {
            session.port = port;
        }
    }

    /// Set the child PID for an active session.
    pub async fn set_child_pid(&self, test_id: &str, pid: u32) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(test_id) {
            session.child_pid = Some(pid);
        }
    }

    /// Get the number of currently active sessions.
    pub async fn active_count(&self) -> usize {
        self.sessions.read().await.len()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> QuotaConfig {
        QuotaConfig {
            max_concurrent_tests: 1,
            max_test_duration_sec: 30,
            max_tests_per_hour_per_peer: 10,
            max_bytes_per_day_per_peer: 10_000_000_000,
            cooldown_sec: 0,
            allow_udp_echo: true,
            allow_throughput: true,
        }
    }

    fn test_params() -> TestParams {
        TestParams {
            duration_sec: 10,
            protocol: None,
            streams: None,
            reverse: None,
        }
    }

    fn make_manager() -> SessionManager {
        let config = test_config();
        let governance = Arc::new(GovernanceEngine::new(config.clone()));
        SessionManager::new(config, governance, "PP-TEST-0000".into())
    }

    #[tokio::test]
    async fn test_request_session_success() {
        let mgr = make_manager();
        let result = mgr
            .request_session("peer-1", TestType::UdpEcho, &test_params())
            .await;
        assert!(result.is_ok());
        let grant = result.unwrap();
        assert!(!grant.test_id.is_empty());
        assert!(!grant.token.is_empty());
        assert_eq!(grant.mode, "direct_ephemeral");
    }

    #[tokio::test]
    async fn test_request_session_busy() {
        let mgr = make_manager();
        // First session should succeed.
        let r1 = mgr
            .request_session("peer-1", TestType::UdpEcho, &test_params())
            .await;
        assert!(r1.is_ok());

        // Second session should be denied (max_concurrent = 1).
        let r2 = mgr
            .request_session("peer-2", TestType::UdpEcho, &test_params())
            .await;
        assert!(r2.is_err());
        let deny = r2.unwrap_err();
        assert_eq!(deny.reason, DenyReason::Busy);
    }

    #[tokio::test]
    async fn test_close_session() {
        let mgr = make_manager();
        let grant = mgr
            .request_session("peer-1", TestType::UdpEcho, &test_params())
            .await
            .unwrap();

        assert_eq!(mgr.active_count().await, 1);
        mgr.close_session(&grant.test_id).await.unwrap();
        assert_eq!(mgr.active_count().await, 0);
    }

    #[tokio::test]
    async fn test_record_bytes() {
        let mgr = make_manager();
        let grant = mgr
            .request_session("peer-1", TestType::UdpEcho, &test_params())
            .await
            .unwrap();

        mgr.record_bytes(&grant.test_id, 1000).await;
        mgr.record_bytes(&grant.test_id, 2000).await;

        let status = mgr.get_status().await;
        assert_eq!(status.bytes_today, 3000);
    }

    #[tokio::test]
    async fn test_cleanup_expired() {
        let config = test_config();
        let governance = Arc::new(GovernanceEngine::new(config.clone()));
        let mgr = SessionManager::new(config, governance, "PP-TEST-0000".into());

        // Request a session.
        let grant = mgr
            .request_session("peer-1", TestType::UdpEcho, &test_params())
            .await
            .unwrap();

        // Manually set the expiry to the past.
        {
            let mut sessions = mgr.sessions.write().await;
            if let Some(session) = sessions.get_mut(&grant.test_id) {
                session.expires_at = Utc::now() - Duration::seconds(10);
            }
        }

        mgr.cleanup_expired().await;
        assert_eq!(mgr.active_count().await, 0);
    }

    #[tokio::test]
    async fn test_get_status() {
        let mgr = make_manager();
        let status = mgr.get_status().await;
        assert_eq!(status.endpoint_id, "PP-TEST-0000");
        assert!(status.active_test.is_none());
    }
}
