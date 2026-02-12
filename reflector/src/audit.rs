//! Structured audit logging for the PacketParamedic Reflector.
//!
//! Every security-relevant event (connections, authorization decisions,
//! sessions, pairing) is appended as a single JSON line to an audit log
//! file.  The log uses `tokio::sync::Mutex` to serialize writes and
//! `tokio::fs::OpenOptions` in append mode for crash safety.

use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use tracing::debug;

// ---------------------------------------------------------------------------
// AuditEventType
// ---------------------------------------------------------------------------

/// Categories of auditable events.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    /// A TLS connection was accepted and the peer identity extracted.
    ConnectionAccepted,
    /// A connection attempt was denied (bad cert, unauthorized peer, etc.).
    ConnectionDenied,
    /// A test session was granted to an authorized peer.
    SessionGranted,
    /// A test session request was denied (quota, cooldown, etc.).
    SessionDenied,
    /// A test session completed (normally or via timeout).
    SessionCompleted,
    /// Pairing mode was enabled on this reflector.
    PairingEnabled,
    /// A new peer was enrolled via the pairing flow.
    PeerPaired,
    /// A peer was removed from the authorized set.
    PeerRemoved,
    /// The reflector's Ed25519 identity was rotated.
    IdentityRotated,
}

// ---------------------------------------------------------------------------
// AuditEntry
// ---------------------------------------------------------------------------

/// A single audit log record.
///
/// Fields that are not applicable to a given event type are set to `None`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// ISO 8601 timestamp of the event.
    pub timestamp: String,
    /// The category of event.
    pub event_type: AuditEventType,
    /// Remote peer endpoint ID, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peer_id: Option<String>,
    /// This reflector's own endpoint ID.
    pub endpoint_id: String,
    /// Test type (e.g. "udp_echo", "throughput"), if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_type: Option<String>,
    /// Unique test / session identifier, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_id: Option<String>,
    /// Arbitrary extra parameters relevant to the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
    /// Authorization decision string (e.g. "allowed", "denied").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<String>,
    /// Human-readable reason for the decision.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Total bytes transferred during the session, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_transferred: Option<u64>,
    /// Duration of the session in seconds, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_sec: Option<f64>,
}

impl AuditEntry {
    /// Create a new `AuditEntry` with the timestamp set to now and all
    /// optional fields set to `None`.
    pub fn new(event_type: AuditEventType, endpoint_id: impl Into<String>) -> Self {
        Self {
            timestamp: Utc::now().to_rfc3339(),
            event_type,
            peer_id: None,
            endpoint_id: endpoint_id.into(),
            test_type: None,
            test_id: None,
            params: None,
            decision: None,
            reason: None,
            bytes_transferred: None,
            duration_sec: None,
        }
    }

    /// Builder-style setter for `peer_id`.
    pub fn with_peer_id(mut self, peer_id: impl Into<String>) -> Self {
        self.peer_id = Some(peer_id.into());
        self
    }

    /// Builder-style setter for `test_type`.
    pub fn with_test_type(mut self, test_type: impl Into<String>) -> Self {
        self.test_type = Some(test_type.into());
        self
    }

    /// Builder-style setter for `test_id`.
    pub fn with_test_id(mut self, test_id: impl Into<String>) -> Self {
        self.test_id = Some(test_id.into());
        self
    }

    /// Builder-style setter for `params`.
    pub fn with_params(mut self, params: serde_json::Value) -> Self {
        self.params = Some(params);
        self
    }

    /// Builder-style setter for `decision`.
    pub fn with_decision(mut self, decision: impl Into<String>) -> Self {
        self.decision = Some(decision.into());
        self
    }

    /// Builder-style setter for `reason`.
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// Builder-style setter for `bytes_transferred`.
    pub fn with_bytes_transferred(mut self, bytes: u64) -> Self {
        self.bytes_transferred = Some(bytes);
        self
    }

    /// Builder-style setter for `duration_sec`.
    pub fn with_duration_sec(mut self, secs: f64) -> Self {
        self.duration_sec = Some(secs);
        self
    }
}

// ---------------------------------------------------------------------------
// AuditLog
// ---------------------------------------------------------------------------

/// Append-only audit log backed by a JSON-lines file.
///
/// Writes are serialized through a `tokio::sync::Mutex` so the log is safe
/// to share across async tasks.
pub struct AuditLog {
    path: PathBuf,
    writer: Mutex<tokio::fs::File>,
}

impl AuditLog {
    /// Open (or create) the audit log file at `path` in append mode.
    pub async fn new(path: PathBuf) -> Result<Self> {
        // Ensure the parent directory exists.
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("failed to create audit log directory: {}", parent.display()))?;
        }

        let file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await
            .with_context(|| format!("failed to open audit log: {}", path.display()))?;

        debug!(path = %path.display(), "audit log opened");

        Ok(Self {
            path,
            writer: Mutex::new(file),
        })
    }

    /// Append a single audit entry as a JSON line.
    pub async fn log(&self, entry: AuditEntry) -> Result<()> {
        let mut line = serde_json::to_string(&entry)
            .context("failed to serialize audit entry")?;
        line.push('\n');

        let mut writer = self.writer.lock().await;
        writer
            .write_all(line.as_bytes())
            .await
            .with_context(|| format!("failed to write to audit log: {}", self.path.display()))?;
        writer
            .flush()
            .await
            .with_context(|| format!("failed to flush audit log: {}", self.path.display()))?;

        Ok(())
    }

    /// Return the path of the audit log file.
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_write_and_read_audit_entries() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("audit.jsonl");

        let log = AuditLog::new(path.clone()).await.unwrap();

        // Write several entries of different types.
        let entry1 = AuditEntry::new(AuditEventType::ConnectionAccepted, "PP-SELF-1234-5678-A")
            .with_peer_id("PP-PEER-AAAA-BBBB-0")
            .with_decision("allowed");

        let entry2 = AuditEntry::new(AuditEventType::SessionGranted, "PP-SELF-1234-5678-A")
            .with_peer_id("PP-PEER-AAAA-BBBB-0")
            .with_test_type("throughput")
            .with_test_id("test-001")
            .with_params(serde_json::json!({"streams": 4, "duration_sec": 10}));

        let entry3 = AuditEntry::new(AuditEventType::SessionCompleted, "PP-SELF-1234-5678-A")
            .with_peer_id("PP-PEER-AAAA-BBBB-0")
            .with_test_type("throughput")
            .with_test_id("test-001")
            .with_bytes_transferred(1_234_567_890)
            .with_duration_sec(10.5);

        log.log(entry1).await.unwrap();
        log.log(entry2).await.unwrap();
        log.log(entry3).await.unwrap();

        // Read the file back and verify JSON-lines format.
        let content = tokio::fs::read_to_string(&path).await.unwrap();
        let lines: Vec<&str> = content.trim().split('\n').collect();
        assert_eq!(lines.len(), 3, "expected 3 JSON lines");

        // Parse each line back.
        let parsed1: AuditEntry = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(parsed1.event_type, AuditEventType::ConnectionAccepted);
        assert_eq!(parsed1.endpoint_id, "PP-SELF-1234-5678-A");
        assert_eq!(parsed1.peer_id.as_deref(), Some("PP-PEER-AAAA-BBBB-0"));
        assert_eq!(parsed1.decision.as_deref(), Some("allowed"));

        let parsed2: AuditEntry = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(parsed2.event_type, AuditEventType::SessionGranted);
        assert_eq!(parsed2.test_type.as_deref(), Some("throughput"));
        assert_eq!(parsed2.test_id.as_deref(), Some("test-001"));
        assert!(parsed2.params.is_some());
        let params = parsed2.params.unwrap();
        assert_eq!(params["streams"], 4);

        let parsed3: AuditEntry = serde_json::from_str(lines[2]).unwrap();
        assert_eq!(parsed3.event_type, AuditEventType::SessionCompleted);
        assert_eq!(parsed3.bytes_transferred, Some(1_234_567_890));
        assert!((parsed3.duration_sec.unwrap() - 10.5).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_audit_log_creates_parent_dirs() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("deep/nested/dir/audit.jsonl");

        let log = AuditLog::new(path.clone()).await.unwrap();
        let entry = AuditEntry::new(AuditEventType::PairingEnabled, "PP-SELF-0000-0000-X");
        log.log(entry).await.unwrap();

        assert!(path.exists());
    }

    #[tokio::test]
    async fn test_audit_entry_builder() {
        let entry = AuditEntry::new(AuditEventType::ConnectionDenied, "PP-SELF-1234-5678-A")
            .with_peer_id("PP-BAD-PEER-0000-Z")
            .with_decision("denied")
            .with_reason("peer not authorized");

        assert_eq!(entry.event_type, AuditEventType::ConnectionDenied);
        assert_eq!(entry.peer_id.as_deref(), Some("PP-BAD-PEER-0000-Z"));
        assert_eq!(entry.decision.as_deref(), Some("denied"));
        assert_eq!(entry.reason.as_deref(), Some("peer not authorized"));
        assert!(entry.test_type.is_none());
        assert!(entry.bytes_transferred.is_none());
    }

    #[tokio::test]
    async fn test_audit_entry_serialization_roundtrip() {
        let entry = AuditEntry::new(AuditEventType::PeerPaired, "PP-SELF-1234-5678-A")
            .with_peer_id("PP-NEW-PEER-1111-B")
            .with_params(serde_json::json!({"method": "token"}));

        let json = serde_json::to_string(&entry).unwrap();
        let parsed: AuditEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.event_type, AuditEventType::PeerPaired);
        assert_eq!(parsed.endpoint_id, "PP-SELF-1234-5678-A");
        assert_eq!(parsed.peer_id.as_deref(), Some("PP-NEW-PEER-1111-B"));
    }

    #[tokio::test]
    async fn test_none_fields_omitted_in_json() {
        let entry = AuditEntry::new(AuditEventType::IdentityRotated, "PP-SELF-1234-5678-A");
        let json = serde_json::to_string(&entry).unwrap();

        // Fields set to None with skip_serializing_if should be absent.
        assert!(!json.contains("\"peer_id\""));
        assert!(!json.contains("\"test_type\""));
        assert!(!json.contains("\"test_id\""));
        assert!(!json.contains("\"params\""));
        assert!(!json.contains("\"decision\""));
        assert!(!json.contains("\"reason\""));
        assert!(!json.contains("\"bytes_transferred\""));
        assert!(!json.contains("\"duration_sec\""));

        // Required fields should be present.
        assert!(json.contains("\"timestamp\""));
        assert!(json.contains("\"event_type\""));
        assert!(json.contains("\"endpoint_id\""));
    }

    #[test]
    fn test_all_event_types_serialize() {
        let types = vec![
            AuditEventType::ConnectionAccepted,
            AuditEventType::ConnectionDenied,
            AuditEventType::SessionGranted,
            AuditEventType::SessionDenied,
            AuditEventType::SessionCompleted,
            AuditEventType::PairingEnabled,
            AuditEventType::PeerPaired,
            AuditEventType::PeerRemoved,
            AuditEventType::IdentityRotated,
        ];

        for t in types {
            let json = serde_json::to_string(&t).unwrap();
            let parsed: AuditEventType = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, t);
        }
    }

    #[tokio::test]
    async fn test_append_mode_preserves_existing() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("audit.jsonl");

        // Write one entry, close the log.
        {
            let log = AuditLog::new(path.clone()).await.unwrap();
            let entry = AuditEntry::new(AuditEventType::ConnectionAccepted, "PP-SELF-0000-0000-X")
                .with_peer_id("PP-PEER-1111-1111-Y");
            log.log(entry).await.unwrap();
        }

        // Re-open and write another entry.
        {
            let log = AuditLog::new(path.clone()).await.unwrap();
            let entry = AuditEntry::new(AuditEventType::ConnectionDenied, "PP-SELF-0000-0000-X")
                .with_peer_id("PP-PEER-2222-2222-Z");
            log.log(entry).await.unwrap();
        }

        // Both entries should be present.
        let content = tokio::fs::read_to_string(&path).await.unwrap();
        let lines: Vec<&str> = content.trim().split('\n').collect();
        assert_eq!(lines.len(), 2, "both entries should be preserved");

        let p1: AuditEntry = serde_json::from_str(lines[0]).unwrap();
        let p2: AuditEntry = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(p1.event_type, AuditEventType::ConnectionAccepted);
        assert_eq!(p2.event_type, AuditEventType::ConnectionDenied);
    }
}
