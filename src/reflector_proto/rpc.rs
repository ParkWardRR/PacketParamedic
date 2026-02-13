
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkMessage {
    pub request_id: String,
    pub payload: MessagePayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessagePayload {
    Hello(Hello),
    ServerHello(ServerHello),
    SessionRequest(SessionRequest),
    SessionGrant(SessionGrant),
    SessionDeny(SessionDeny),
    SessionClose(SessionClose),
    GetStatus,
    StatusSnapshot(StatusSnapshot),
    PairRequest(PairRequest),
    PairResponse(PairResponse),
    GetPathMeta,
    PathMeta(PathMeta),
    Ok,
    Error(ErrorResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hello {
    pub version: String,
    pub features: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerHello {
    pub version: String,
    pub features: Vec<String>,
    pub policy_summary: PolicySummary,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network_position: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicySummary {
    pub max_test_duration_sec: u64,
    pub max_concurrent_tests: u32,
    pub max_tests_per_hour: u32,
    pub allowed_test_types: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRequest {
    pub test_type: TestType,
    pub params: TestParams,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TestType {
    Throughput,
    UdpEcho,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestParams {
    pub duration_sec: u64,
    pub protocol: Option<String>,
    pub streams: Option<u32>,
    pub reverse: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionGrant {
    pub test_id: String,
    pub mode: String,
    pub port: u16,
    pub token: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDeny {
    pub reason: DenyReason,
    pub message: String,
    pub retry_after_sec: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DenyReason {
    Unauthorized,
    RateLimited,
    Busy,
    InvalidParams,
    QuotaExceeded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionClose {
    pub test_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairRequest {
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairResponse {
    pub success: bool,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusSnapshot {
    pub endpoint_id: String,
    pub uptime_sec: u64,
    pub active_test: Option<ActiveTestInfo>,
    pub tests_today: u32,
    pub bytes_today: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network_position: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveTestInfo {
    pub test_id: String,
    pub test_type: TestType,
    pub peer_id: String,
    pub started_at: String,
    pub remaining_sec: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathMeta {
    pub cpu_load: f64,
    pub memory_used_mb: u64,
    pub memory_total_mb: u64,
    pub load_avg: [f64; 3],
    pub mtu: Option<u32>,
    pub time_synced: bool,
    pub build_version: String,
    pub build_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub code: u32,
    pub message: String,
}
