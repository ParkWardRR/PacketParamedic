use crate::storage::Pool;
use crate::detect::anomaly::TimeSeries;
use crate::detect::incident::IncidentManager;
use crate::detect::Severity;
use anyhow::{Context, Result};
use rusqlite::params;
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

    /// Run a scan for anomalies on key metrics.
    /// This should be called periodically (e.g. every 5-10 mins).
    pub async fn run_scan(&self) -> Result<()> {
        info!("Running anomaly detection scan");
        self.check_icmp_latency().await?;
        Ok(())
    }

    /// Check for ICMP latency spikes (Z-Score > 3.0) across all targets
    async fn check_icmp_latency(&self) -> Result<()> {
        let pool = self.pool.clone();
        
        // Spawn blocking task for DB query & analysis
        let anomalies: Vec<(String, f64, f64, f64)> = tokio::task::spawn_blocking(move || -> Result<Vec<(String, f64, f64, f64)>> {
            let conn = pool.get()?;
            let mut stmt = conn.prepare(
                "SELECT target, latency_ms FROM probe_results 
                 WHERE probe_type = 'icmp' 
                 AND created_at > datetime('now', '-1 hour')
                 ORDER BY created_at ASC"
            )?;
            
            use std::collections::HashMap;
            let mut samples: HashMap<String, Vec<f64>> = HashMap::new();
            
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
            })?;

            for r in rows {
                let (target, latency) = r?;
                samples.entry(target).or_default().push(latency);
            }

            let mut results = Vec::new();
            for (target, values) in samples {
                if values.len() < 10 { continue; }
                
                let ts = TimeSeries::new(values.clone());
                if let Some(last_val) = values.last().copied() {
                    let mean = ts.mean();
                    // Z-Score calc
                    if let Ok(z) = ts.z_score(last_val) {
                        if z > 3.0 && last_val > 20.0 { 
                             results.push((target, last_val, mean, z));
                        }
                    }
                }
            }
            Ok(results)
        }).await??;

        // Record any found anomalies
        for (target, val, mean, z) in anomalies {
            let msg = format!("Latency spike detected on {}: {:.1}ms (Baseline: {:.1}ms, Z-Score: {:.1})", target, val, mean, z);
            warn!("{}", msg);
            
            self.incident_manager.record_incident(
                "Latency Anomaly",
                Severity::Warning,
                serde_json::json!({
                    "target": target,
                    "metric": "latency",
                    "current": val,
                    "mean": mean,
                    "z_score": z,
                    "description": msg
                })
            )?;
        }
        
        Ok(())
    }
}
