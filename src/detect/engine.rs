use crate::storage::Pool;
use crate::detect::incident::IncidentManager;
use crate::analysis::stats::check_for_anomaly;
use anyhow::Result;
use tracing::{info, warn};

pub struct AnomalyEngine {
    pool: Pool,
    incident_manager: IncidentManager,
}

impl AnomalyEngine {
    pub fn new(pool: Pool) -> Self {
        let incident_manager = IncidentManager::new(pool.clone());
        Self { pool, incident_manager }
    }

    /// Run a scan for anomalies.
    /// This iterates over known targets and checks their latest value against the baseline.
    /// Typically called by a cron schedule (e.g. "anomaly-scan").
    pub async fn run_scan(&self) -> Result<()> {
        info!("Running anomaly detection scan");
        
        let pool = self.pool.clone();
        
        // Find all active targets in the last hour
        let targets: Vec<(String, String)> = tokio::task::spawn_blocking(move || -> Result<Vec<(String, String)>> {
            let conn = pool.get()?;
            let mut stmt = conn.prepare(
                "SELECT DISTINCT probe_type, target FROM measurements 
                 WHERE created_at > datetime('now', '-1 hour')"
            )?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            
            let mut res = Vec::new();
            for r in rows { res.push(r?); }
            Ok(res)
        }).await??;

        for (probe_type, target) in targets {
            self.analyze_target(&probe_type, &target).await?;
        }
        
        Ok(())
    }

    async fn analyze_target(&self, probe_type: &str, target: &str) -> Result<()> {
        let pool = self.pool.clone();
        let pt = probe_type.to_string();
        let t = target.to_string();

        // Fetch latest measurement
        let latest: Option<(f64, String)> = tokio::task::spawn_blocking(move || -> Result<Option<(f64, String)>> {
            let conn = pool.get()?;
            let mut stmt = conn.prepare(
                "SELECT value, created_at FROM measurements 
                 WHERE probe_type = ?1 AND target = ?2 
                 ORDER BY created_at DESC LIMIT 1"
            )?;
            
            let mut rows = stmt.query_map(rusqlite::params![pt, t], |row| {
                Ok((row.get::<_, f64>(0)?, row.get::<_, String>(1)?))
            })?;
            
            if let Some(r) = rows.next() {
                Ok(Some(r?))
            } else {
                Ok(None)
            }
        }).await??;

        if let Some((val, _ts)) = latest {
            // Check for anomaly
            let pool = self.pool.clone();
            let pt = probe_type.to_string();
            let t = target.to_string();
            
            // Just use the stats engine!
            let anomaly_opt = tokio::task::spawn_blocking(move || {
                check_for_anomaly(&pool, &pt, &t, val)
            }).await??;

            if let Some(anomaly) = anomaly_opt {
                warn!(target=%anomaly.target, "Anomaly Detected: {:?}", anomaly);
                self.incident_manager.record_incident(
                    &format!("{} Anomaly: {}", anomaly.probe_type.to_uppercase(), anomaly.target),
                    anomaly.severity,
                    serde_json::json!({
                        "target": anomaly.target,
                        "probe_type": anomaly.probe_type,
                        "value": anomaly.value,
                        "baseline_mean": anomaly.baseline_mean,
                        "z_score": anomaly.z_score,
                        "val_unit": "ms"
                    })
                )?;
            }
        }
        Ok(())
    }
}
