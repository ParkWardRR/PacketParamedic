use crate::storage::Pool;
use crate::detect::incident::IncidentManager;
use crate::detect::Severity;
use crate::system::network;
use anyhow::Result;
use tracing::{info, warn};
use std::collections::HashMap;

/// Correlates independent anomalies into a root cause.
pub struct CorrelationEngine {
    pool: Pool,
    incident_manager: IncidentManager,
}

#[derive(Debug, PartialEq, Eq)]
pub enum RootCause {
    LocalNetwork, // Gateway is bad -> It's you.
    IspIssue,     // Gateway good, multiple WAN targets bad -> It's them.
    RemoteService,// Only one WAN target bad -> It's Google/Netflix.
    Unknown,      // Not enough data.
}

impl CorrelationEngine {
    pub fn new(pool: Pool) -> Self {
        let incident_manager = IncidentManager::new(pool.clone());
        Self { pool, incident_manager }
    }

    /// Run the correlation logic for the last N minutes.
    pub async fn correlate(&self) -> Result<()> {
        // 2. Identify the Gateway.
        let gateway = network::get_default_gateway().unwrap_or_else(|_| "192.168.1.1".to_string());
        self.correlate_with_gateway(&gateway).await
    }

    /// Internal correlation logic with explicit gateway target.
    pub async fn correlate_with_gateway(&self, gateway: &str) -> Result<()> {
        let conn = self.pool.get()?;
        
        // 1. Get all OPEN incidents from the last 15 minutes.
        let mut stmt = conn.prepare(
            "SELECT id, verdict, evidence_json FROM incidents 
             WHERE status = 'Open' 
             AND updated_at > datetime('now', '-15 minutes')"
        )?;
        
        let mut anomalous_targets = HashMap::new();
        
        let rows = stmt.query_map([], |row| {
             let verdict: String = row.get(1)?;
             let evidence_str: String = row.get(2)?;
             let evidence: serde_json::Value = serde_json::from_str(&evidence_str).unwrap_or_default();
             Ok((verdict, evidence))
        })?;

        for r in rows {
            let (verdict, evidence) = r?;
            if verdict.contains("Anomaly") {
                if let Some(target) = evidence.get("target").and_then(|t| t.as_str()) {
                    anomalous_targets.insert(target.to_string(), evidence);
                }
            }
        }

        if anomalous_targets.is_empty() {
            return Ok(()); // No active anomalies to correlate.
        }

        // 3. Analyze Patterns
        let gateway_bad = anomalous_targets.contains_key(gateway);
        let wan_bad_count = anomalous_targets.keys().filter(|k| *k != gateway).count();
        
        let root_cause = if gateway_bad {
            RootCause::LocalNetwork
        } else if wan_bad_count > 1 {
            RootCause::IspIssue
        } else if wan_bad_count == 1 {
            RootCause::RemoteService
        } else {
            RootCause::Unknown
        };

        if root_cause != RootCause::Unknown {
            self.publish_correlation_incident(root_cause, &anomalous_targets, gateway)?;
        }

        Ok(())
    }

    fn publish_correlation_incident(
        &self, 
        cause: RootCause, 
        details: &HashMap<String, serde_json::Value>, 
        gateway: &str
    ) -> Result<()> {
        let (verdict, explanation, severity) = match cause {
            RootCause::LocalNetwork => (
                "Local Network Issue", 
                format!("High latency detected on Gateway ({}). Likely Wi-Fi interference or router overload.", gateway),
                Severity::Warning
            ),
            RootCause::IspIssue => (
                "ISP Network Issue",
                "Gateway is healthy, but multiple internet services are suffering connectivity issues.".to_string(),
                Severity::Critical // ISP issues are usually what users care about most
            ),
            RootCause::RemoteService => (
                "Specific Service Issue",
                "Gateway and other services are fine. Only one target is affected.".to_string(),
                Severity::Info
            ),
            RootCause::Unknown => return Ok(()),
        };

        info!("Correlation Verdict: {} ({})", verdict, explanation);

        // Record the Master Incident
        self.incident_manager.record_incident(
            verdict,
            severity,
            serde_json::json!({
                "cause": format!("{:?}", cause),
                "explanation": explanation,
                "related_anomalies": details.len(),
                "gateway": gateway,
                "affected_targets": details.keys().collect::<Vec<_>>()
            })
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Add logic to test correlation rules
    
    #[tokio::test]
    async fn test_local_issue_correlation() -> Result<()> {
        use std::time::SystemTime;
        let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos();
        let db_name = format!("test_coerr_{}.db", now);
        
        // 1. Setup DB
        let manager = r2d2_sqlite::SqliteConnectionManager::file(&db_name);
        let pool = r2d2::Pool::new(manager)?;
        let conn = pool.get()?;
        crate::storage::schema::migrate(&conn)?;
        drop(conn);

        // 2. Seed Incidents
        let im = IncidentManager::new(pool.clone());
        
        // Mock gateway
        let gateway = "192.168.1.1";
        
        // Incident 1: High Latency on Gateway
        im.record_incident(
            "Latency Anomaly: 192.168.1.1",
            Severity::Warning,
            serde_json::json!({ "target": gateway, "metric": "latency", "z_score": 4.5 })
        )?;

        // Incident 2: High Latency on WAN Target A
        im.record_incident(
            "Latency Anomaly: 8.8.8.8",
            Severity::Warning,
            serde_json::json!({ "target": "8.8.8.8", "metric": "latency", "z_score": 5.0 })
        )?;

        // Incident 3: High Latency on WAN Target B
        im.record_incident(
            "Latency Anomaly: 1.1.1.1",
            Severity::Warning,
            serde_json::json!({ "target": "1.1.1.1", "metric": "latency", "z_score": 3.2 })
        )?;
        
        // 3. Run Correlation
        // We must mock network::get_default_gateway or ensure logic uses our mocked gateway.
        // The implementation calls network::get_default_gateway().
        // If the test runs where 192.168.1.1 is NOT the gateway, logic might fail.
        // However, get_default_gateway falls back to 192.168.1.1 on failure.
        // If test runs on mac/linux without 'ip' command access or route, it falls back.
        // If it returns something else (e.g. 10.0.0.1), then our seed data won't match.
        // For unit test stability, we should probably dependency-inject the gateway.
        // But for now, let's assume fallback or modify CorrelationEngine to accept gateway override.
        // Let's modify the engine slightly to allow injection.
        // Or just rely on fallback.
        
        let engine = CorrelationEngine::new(pool.clone());
        // We act on live DB. 
        // If get_default_gateway returns 192.168.1.1, we expect LocalNetwork.
        
        // Let's invoke correlate
        // But wait, correlate() does: let gateway = network::get_default_gateway()...
        // I can't easily mock that function.
        // I'll make `correlate_with_gateway(&self, gateway: &str)` public for testing?
        // Yes, refactoring slightly.
        engine.correlate_with_gateway(gateway).await?;
        
        // 4. Verify Result
        let incidents = im.list_recent(5)?;
        assert!(incidents.len() >= 4); // 3 original + 1 correlated
        
        let correlated = incidents.iter().find(|i| i.verdict == "Local Network Issue");
        assert!(correlated.is_some(), "Should detect Local Network Issue");
        let i = correlated.unwrap();
        assert_eq!(i.severity, Severity::Warning);
        
        // Clean up
        let _ = std::fs::remove_file(&db_name);
        Ok(())
    }
}
