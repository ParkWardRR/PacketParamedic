//! RPC message types for the Paramedic Link protocol.
//!
//! All messages are wrapped in a [`LinkMessage`] envelope that carries a unique
//! `request_id` and a tagged [`MessagePayload`] discriminant.  The payload is
//! serialized as internally-tagged JSON (`"type": "..."`) so that the receiver
//! can dispatch on the `type` field without parsing the full body first.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Top-level envelope
// ---------------------------------------------------------------------------

/// Top-level envelope for every message on the Paramedic Link control channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkMessage {
    /// Unique identifier for correlating requests with responses.
    pub request_id: String,
    /// The actual message content.
    pub payload: MessagePayload,
}

/// Discriminated union of all Paramedic Link message types.
///
/// Serialized as internally-tagged JSON: `{ "type": "<variant>", ... }`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessagePayload {
    // -- Capability negotiation --
    Hello(Hello),
    ServerHello(ServerHello),

    // -- Session management --
    SessionRequest(SessionRequest),
    SessionGrant(SessionGrant),
    SessionDeny(SessionDeny),
    SessionClose(SessionClose),

    // -- Status --
    GetStatus,
    StatusSnapshot(StatusSnapshot),

    // -- Pairing --
    PairRequest(PairRequest),
    PairResponse(PairResponse),

    // -- Meta --
    GetPathMeta,
    PathMeta(PathMeta),

    // -- Generic --
    Ok,
    Error(ErrorResponse),
}

// ---------------------------------------------------------------------------
// Capability negotiation
// ---------------------------------------------------------------------------

/// Sent by the client immediately after the TLS handshake.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hello {
    /// Protocol version, e.g. `"1.0"`.
    pub version: String,
    /// Capabilities the client supports, e.g. `["throughput", "udp_echo", "path_meta"]`.
    pub features: Vec<String>,
}

/// Sent by the server in response to [`Hello`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerHello {
    /// Protocol version, e.g. `"1.0"`.
    pub version: String,
    /// Capabilities the server supports.
    pub features: Vec<String>,
    /// Summary of the server's rate-limiting and resource policies.
    pub policy_summary: PolicySummary,
    /// Network position of this reflector (`"wan"`, `"lan"`, `"hybrid"`, or `"unknown"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network_position: Option<String>,
}

/// Summary of server-side resource policies communicated during the handshake.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicySummary {
    /// Maximum duration (seconds) for a single test.
    pub max_test_duration_sec: u64,
    /// Maximum number of tests that may run simultaneously.
    pub max_concurrent_tests: u32,
    /// Maximum number of tests allowed per rolling hour window.
    pub max_tests_per_hour: u32,
    /// Test types the server is willing to run.
    pub allowed_test_types: Vec<String>,
}

// ---------------------------------------------------------------------------
// Session management
// ---------------------------------------------------------------------------

/// Client request to start a new test session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRequest {
    /// The kind of test to run.
    pub test_type: TestType,
    /// Parameters for the test.
    pub params: TestParams,
}

/// Supported test types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TestType {
    Throughput,
    UdpEcho,
}

/// Parameters that tune a test session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestParams {
    /// How long the test should run (seconds).
    pub duration_sec: u64,
    /// `"tcp"` or `"udp"` (throughput tests only).
    pub protocol: Option<String>,
    /// Number of parallel streams (throughput tests only).
    pub streams: Option<u32>,
    /// Whether to run in reverse mode (throughput tests only).
    pub reverse: Option<bool>,
}

/// Server grants a test session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionGrant {
    /// Unique test identifier (UUID).
    pub test_id: String,
    /// `"tunneled"` or `"direct_ephemeral"`.
    pub mode: String,
    /// Data-plane port to connect to.
    pub port: u16,
    /// One-time auth cookie for the data channel.
    pub token: String,
    /// ISO 8601 expiration timestamp for this grant.
    pub expires_at: String,
}

/// Server denies a test session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDeny {
    /// Machine-readable denial reason.
    pub reason: DenyReason,
    /// Human-readable explanation.
    pub message: String,
    /// Hint for the client to retry after this many seconds.
    pub retry_after_sec: Option<u64>,
}

/// Reasons why a session may be denied.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DenyReason {
    Unauthorized,
    RateLimited,
    Busy,
    InvalidParams,
    QuotaExceeded,
}

/// Client or server signals that a test session has ended.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionClose {
    /// The test that should be torn down.
    pub test_id: String,
}

// ---------------------------------------------------------------------------
// Pairing
// ---------------------------------------------------------------------------

/// Request to pair with the reflector using a temporary code.
///
/// Either side can generate the 8-digit alphanumeric code. The connecting
/// peer sends this message with the code to establish a long-term pairing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairRequest {
    /// The 8-digit alphanumeric pairing code.
    pub token: String,
}

/// Response to a pairing attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairResponse {
    /// Whether the pairing was successful.
    pub success: bool,
    /// Human-readable result message.
    pub message: String,
    /// The reflector's endpoint ID (included on success for the peer to store).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Status
// ---------------------------------------------------------------------------

/// Snapshot of the reflector's current state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusSnapshot {
    /// The reflector's endpoint identity.
    pub endpoint_id: String,
    /// Seconds since the reflector process started.
    pub uptime_sec: u64,
    /// Currently running test, if any.
    pub active_test: Option<ActiveTestInfo>,
    /// Number of tests completed in the current UTC day.
    pub tests_today: u32,
    /// Total bytes transferred in the current UTC day.
    pub bytes_today: u64,
    /// Network position of this reflector (`"wan"`, `"lan"`, `"hybrid"`, or `"unknown"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network_position: Option<String>,
}

/// Information about a currently running test.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveTestInfo {
    /// Test identifier.
    pub test_id: String,
    /// Kind of test.
    pub test_type: TestType,
    /// Identity of the remote peer.
    pub peer_id: String,
    /// ISO 8601 timestamp when the test started.
    pub started_at: String,
    /// Seconds remaining until the test's scheduled end.
    pub remaining_sec: u64,
}

// ---------------------------------------------------------------------------
// Path meta
// ---------------------------------------------------------------------------

/// System and network metadata useful for path characterization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathMeta {
    /// CPU load as a fraction (0.0 - 1.0+).
    pub cpu_load: f64,
    /// Resident memory usage in MiB.
    pub memory_used_mb: u64,
    /// Total physical memory in MiB.
    pub memory_total_mb: u64,
    /// 1-minute, 5-minute, and 15-minute load averages.
    pub load_avg: [f64; 3],
    /// Path MTU if known.
    pub mtu: Option<u32>,
    /// Whether the system clock is synchronized (e.g. via NTP).
    pub time_synced: bool,
    /// Semantic version of the reflector build.
    pub build_version: String,
    /// Git commit hash of the reflector build.
    pub build_hash: String,
}

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

/// Generic error response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Numeric error code.
    pub code: u32,
    /// Human-readable error description.
    pub message: String,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: serialize to JSON, then deserialize back, returning both the
    /// intermediate JSON string and the round-tripped value.
    fn round_trip<T: Serialize + serde::de::DeserializeOwned + std::fmt::Debug>(
        val: &T,
    ) -> (String, T) {
        let json = serde_json::to_string_pretty(val).expect("serialize");
        let back: T = serde_json::from_str(&json).expect("deserialize");
        (json, back)
    }

    #[test]
    fn test_hello_round_trip() {
        let msg = LinkMessage {
            request_id: "req-001".into(),
            payload: MessagePayload::Hello(Hello {
                version: "1.0".into(),
                features: vec![
                    "throughput".into(),
                    "udp_echo".into(),
                    "path_meta".into(),
                ],
            }),
        };
        let (json, decoded) = round_trip(&msg);
        assert!(json.contains(r#""type": "hello""#));
        assert_eq!(decoded.request_id, "req-001");
        match &decoded.payload {
            MessagePayload::Hello(h) => {
                assert_eq!(h.version, "1.0");
                assert_eq!(h.features.len(), 3);
            }
            other => panic!("expected Hello, got {:?}", other),
        }
    }

    #[test]
    fn test_server_hello_round_trip() {
        let msg = LinkMessage {
            request_id: "req-002".into(),
            payload: MessagePayload::ServerHello(ServerHello {
                version: "1.0".into(),
                features: vec!["throughput".into()],
                policy_summary: PolicySummary {
                    max_test_duration_sec: 60,
                    max_concurrent_tests: 1,
                    max_tests_per_hour: 10,
                    allowed_test_types: vec!["throughput".into()],
                },
                network_position: Some("wan".into()),
            }),
        };
        let (json, decoded) = round_trip(&msg);
        assert!(json.contains(r#""type": "server_hello""#));
        match &decoded.payload {
            MessagePayload::ServerHello(sh) => {
                assert_eq!(sh.policy_summary.max_test_duration_sec, 60);
                assert_eq!(sh.policy_summary.max_concurrent_tests, 1);
            }
            other => panic!("expected ServerHello, got {:?}", other),
        }
    }

    #[test]
    fn test_session_request_round_trip() {
        let msg = LinkMessage {
            request_id: "req-003".into(),
            payload: MessagePayload::SessionRequest(SessionRequest {
                test_type: TestType::Throughput,
                params: TestParams {
                    duration_sec: 30,
                    protocol: Some("tcp".into()),
                    streams: Some(4),
                    reverse: Some(false),
                },
            }),
        };
        let (json, decoded) = round_trip(&msg);
        assert!(json.contains(r#""type": "session_request""#));
        match &decoded.payload {
            MessagePayload::SessionRequest(sr) => {
                assert_eq!(sr.test_type, TestType::Throughput);
                assert_eq!(sr.params.duration_sec, 30);
                assert_eq!(sr.params.protocol.as_deref(), Some("tcp"));
                assert_eq!(sr.params.streams, Some(4));
                assert_eq!(sr.params.reverse, Some(false));
            }
            other => panic!("expected SessionRequest, got {:?}", other),
        }
    }

    #[test]
    fn test_session_grant_round_trip() {
        let msg = LinkMessage {
            request_id: "req-004".into(),
            payload: MessagePayload::SessionGrant(SessionGrant {
                test_id: "550e8400-e29b-41d4-a716-446655440000".into(),
                mode: "tunneled".into(),
                port: 5201,
                token: "abc123secret".into(),
                expires_at: "2025-06-15T12:00:00Z".into(),
            }),
        };
        let (json, decoded) = round_trip(&msg);
        assert!(json.contains(r#""type": "session_grant""#));
        match &decoded.payload {
            MessagePayload::SessionGrant(sg) => {
                assert_eq!(sg.port, 5201);
                assert_eq!(sg.mode, "tunneled");
            }
            other => panic!("expected SessionGrant, got {:?}", other),
        }
    }

    #[test]
    fn test_session_deny_round_trip() {
        let msg = LinkMessage {
            request_id: "req-005".into(),
            payload: MessagePayload::SessionDeny(SessionDeny {
                reason: DenyReason::RateLimited,
                message: "Too many tests this hour".into(),
                retry_after_sec: Some(120),
            }),
        };
        let (json, decoded) = round_trip(&msg);
        assert!(json.contains(r#""type": "session_deny""#));
        assert!(json.contains(r#""reason": "rate_limited""#));
        match &decoded.payload {
            MessagePayload::SessionDeny(sd) => {
                assert_eq!(sd.reason, DenyReason::RateLimited);
                assert_eq!(sd.retry_after_sec, Some(120));
            }
            other => panic!("expected SessionDeny, got {:?}", other),
        }
    }

    #[test]
    fn test_session_close_round_trip() {
        let msg = LinkMessage {
            request_id: "req-006".into(),
            payload: MessagePayload::SessionClose(SessionClose {
                test_id: "test-42".into(),
            }),
        };
        let (json, decoded) = round_trip(&msg);
        assert!(json.contains(r#""type": "session_close""#));
        match &decoded.payload {
            MessagePayload::SessionClose(sc) => assert_eq!(sc.test_id, "test-42"),
            other => panic!("expected SessionClose, got {:?}", other),
        }
    }

    #[test]
    fn test_get_status_round_trip() {
        let msg = LinkMessage {
            request_id: "req-007".into(),
            payload: MessagePayload::GetStatus,
        };
        let (json, decoded) = round_trip(&msg);
        assert!(json.contains(r#""type": "get_status""#));
        assert!(matches!(decoded.payload, MessagePayload::GetStatus));
    }

    #[test]
    fn test_status_snapshot_round_trip() {
        let msg = LinkMessage {
            request_id: "req-008".into(),
            payload: MessagePayload::StatusSnapshot(StatusSnapshot {
                endpoint_id: "pp1abc".into(),
                uptime_sec: 3600,
                active_test: Some(ActiveTestInfo {
                    test_id: "test-99".into(),
                    test_type: TestType::UdpEcho,
                    peer_id: "pp1xyz".into(),
                    started_at: "2025-06-15T11:30:00Z".into(),
                    remaining_sec: 25,
                }),
                tests_today: 5,
                bytes_today: 1_000_000_000,
                network_position: Some("wan".into()),
            }),
        };
        let (json, decoded) = round_trip(&msg);
        assert!(json.contains(r#""type": "status_snapshot""#));
        match &decoded.payload {
            MessagePayload::StatusSnapshot(ss) => {
                assert_eq!(ss.uptime_sec, 3600);
                assert!(ss.active_test.is_some());
                let at = ss.active_test.as_ref().unwrap();
                assert_eq!(at.test_type, TestType::UdpEcho);
                assert_eq!(at.remaining_sec, 25);
            }
            other => panic!("expected StatusSnapshot, got {:?}", other),
        }
    }

    #[test]
    fn test_get_path_meta_round_trip() {
        let msg = LinkMessage {
            request_id: "req-009".into(),
            payload: MessagePayload::GetPathMeta,
        };
        let (json, decoded) = round_trip(&msg);
        assert!(json.contains(r#""type": "get_path_meta""#));
        assert!(matches!(decoded.payload, MessagePayload::GetPathMeta));
    }

    #[test]
    fn test_path_meta_round_trip() {
        let msg = LinkMessage {
            request_id: "req-010".into(),
            payload: MessagePayload::PathMeta(PathMeta {
                cpu_load: 0.35,
                memory_used_mb: 512,
                memory_total_mb: 2048,
                load_avg: [0.25, 0.30, 0.28],
                mtu: Some(1500),
                time_synced: true,
                build_version: "0.1.0".into(),
                build_hash: "abcdef1234567890".into(),
            }),
        };
        let (json, decoded) = round_trip(&msg);
        assert!(json.contains(r#""type": "path_meta""#));
        match &decoded.payload {
            MessagePayload::PathMeta(pm) => {
                assert!((pm.cpu_load - 0.35).abs() < f64::EPSILON);
                assert_eq!(pm.memory_used_mb, 512);
                assert_eq!(pm.mtu, Some(1500));
                assert!(pm.time_synced);
            }
            other => panic!("expected PathMeta, got {:?}", other),
        }
    }

    #[test]
    fn test_ok_round_trip() {
        let msg = LinkMessage {
            request_id: "req-011".into(),
            payload: MessagePayload::Ok,
        };
        let (json, decoded) = round_trip(&msg);
        assert!(json.contains(r#""type": "ok""#));
        assert!(matches!(decoded.payload, MessagePayload::Ok));
    }

    #[test]
    fn test_error_round_trip() {
        let msg = LinkMessage {
            request_id: "req-012".into(),
            payload: MessagePayload::Error(ErrorResponse {
                code: 400,
                message: "invalid test parameters".into(),
            }),
        };
        let (json, decoded) = round_trip(&msg);
        assert!(json.contains(r#""type": "error""#));
        match &decoded.payload {
            MessagePayload::Error(e) => {
                assert_eq!(e.code, 400);
                assert_eq!(e.message, "invalid test parameters");
            }
            other => panic!("expected Error, got {:?}", other),
        }
    }

    #[test]
    fn test_test_type_serialization() {
        let throughput = serde_json::to_string(&TestType::Throughput).unwrap();
        assert_eq!(throughput, r#""throughput""#);

        let udp_echo = serde_json::to_string(&TestType::UdpEcho).unwrap();
        assert_eq!(udp_echo, r#""udp_echo""#);

        // Round-trip
        let back: TestType = serde_json::from_str(&throughput).unwrap();
        assert_eq!(back, TestType::Throughput);
    }

    #[test]
    fn test_deny_reason_serialization() {
        let reasons = vec![
            (DenyReason::Unauthorized, "unauthorized"),
            (DenyReason::RateLimited, "rate_limited"),
            (DenyReason::Busy, "busy"),
            (DenyReason::InvalidParams, "invalid_params"),
            (DenyReason::QuotaExceeded, "quota_exceeded"),
        ];
        for (variant, expected) in reasons {
            let json = serde_json::to_string(&variant).unwrap();
            assert_eq!(json, format!(r#""{}""#, expected));
            let back: DenyReason = serde_json::from_str(&json).unwrap();
            assert_eq!(back, variant);
        }
    }

    #[test]
    fn test_status_snapshot_no_active_test() {
        let msg = LinkMessage {
            request_id: "req-013".into(),
            payload: MessagePayload::StatusSnapshot(StatusSnapshot {
                endpoint_id: "pp1idle".into(),
                uptime_sec: 7200,
                active_test: None,
                tests_today: 0,
                bytes_today: 0,
                network_position: None,
            }),
        };
        let (_, decoded) = round_trip(&msg);
        match &decoded.payload {
            MessagePayload::StatusSnapshot(ss) => {
                assert!(ss.active_test.is_none());
            }
            other => panic!("expected StatusSnapshot, got {:?}", other),
        }
    }

    #[test]
    fn test_session_request_udp_echo_minimal_params() {
        let msg = LinkMessage {
            request_id: "req-014".into(),
            payload: MessagePayload::SessionRequest(SessionRequest {
                test_type: TestType::UdpEcho,
                params: TestParams {
                    duration_sec: 10,
                    protocol: None,
                    streams: None,
                    reverse: None,
                },
            }),
        };
        let (json, decoded) = round_trip(&msg);
        // Optional fields should serialize as null.
        assert!(json.contains(r#""protocol": null"#));
        match &decoded.payload {
            MessagePayload::SessionRequest(sr) => {
                assert_eq!(sr.test_type, TestType::UdpEcho);
                assert!(sr.params.protocol.is_none());
            }
            other => panic!("expected SessionRequest, got {:?}", other),
        }
    }

    #[test]
    fn test_pair_request_round_trip() {
        let msg = LinkMessage {
            request_id: "req-pair-1".into(),
            payload: MessagePayload::PairRequest(PairRequest {
                token: "ABCD1234".into(),
            }),
        };
        let (json, decoded) = round_trip(&msg);
        assert!(json.contains(r#""type": "pair_request""#));
        match &decoded.payload {
            MessagePayload::PairRequest(pr) => assert_eq!(pr.token, "ABCD1234"),
            other => panic!("expected PairRequest, got {:?}", other),
        }
    }

    #[test]
    fn test_pair_response_success_round_trip() {
        let msg = LinkMessage {
            request_id: "req-pair-2".into(),
            payload: MessagePayload::PairResponse(PairResponse {
                success: true,
                message: "paired successfully".into(),
                endpoint_id: Some("PP-ABCD-EFGH-IJKL-0".into()),
            }),
        };
        let (json, decoded) = round_trip(&msg);
        assert!(json.contains(r#""type": "pair_response""#));
        match &decoded.payload {
            MessagePayload::PairResponse(pr) => {
                assert!(pr.success);
                assert_eq!(pr.endpoint_id.as_deref(), Some("PP-ABCD-EFGH-IJKL-0"));
            }
            other => panic!("expected PairResponse, got {:?}", other),
        }
    }

    #[test]
    fn test_pair_response_failure_round_trip() {
        let msg = LinkMessage {
            request_id: "req-pair-3".into(),
            payload: MessagePayload::PairResponse(PairResponse {
                success: false,
                message: "invalid or expired pairing code".into(),
                endpoint_id: None,
            }),
        };
        let (_, decoded) = round_trip(&msg);
        match &decoded.payload {
            MessagePayload::PairResponse(pr) => {
                assert!(!pr.success);
                assert!(pr.endpoint_id.is_none());
            }
            other => panic!("expected PairResponse, got {:?}", other),
        }
    }

    #[test]
    fn test_session_deny_no_retry() {
        let msg = LinkMessage {
            request_id: "req-015".into(),
            payload: MessagePayload::SessionDeny(SessionDeny {
                reason: DenyReason::Unauthorized,
                message: "peer not in allowlist".into(),
                retry_after_sec: None,
            }),
        };
        let (_, decoded) = round_trip(&msg);
        match &decoded.payload {
            MessagePayload::SessionDeny(sd) => {
                assert_eq!(sd.reason, DenyReason::Unauthorized);
                assert!(sd.retry_after_sec.is_none());
            }
            other => panic!("expected SessionDeny, got {:?}", other),
        }
    }
}
