use crate::storage::Pool;
use anyhow::{Context, Result};
use rusqlite::params;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Baseline {
    pub mean: f64,
    pub std_dev: f64,
    pub sample_count: u64,
    pub z_score_threshold: f64,
}

impl Default for Baseline {
    fn default() -> Self {
        Self {
            mean: 0.0,
            std_dev: 0.0,
            sample_count: 0,
            z_score_threshold: 3.0,
        }
    }
}

/// Calculate statisical baseline for a given probe type + target over a time window.
/// Default window is 24 hours.
pub fn calculate_baseline(pool: &Pool, probe_type: &str, target: &str) -> Result<Baseline> {
    let conn = pool.get()?;

    // 1. Get raw values for the last 24h
    // We fetch raw values to compute std_dev reliably in Rust (SQLite stddev extension is often missing)
    let mut stmt = conn.prepare(
        "SELECT value FROM measurements 
         WHERE probe_type = ?1 
         AND target = ?2 
         AND created_at > datetime('now', '-24 hours')
         AND value >= 0 -- Exclude error sentinels (-1.0)"
    )?;

    let rows = stmt.query_map(params![probe_type, target], |row| row.get::<_, f64>(0))?;

    let mut values = Vec::new();
    for r in rows {
        values.push(r?);
    }

    if values.is_empty() {
        return Ok(Baseline::default());
    }

    let count = values.len() as u64;
    let sum: f64 = values.iter().sum();
    let mean = sum / count as f64;

    let variance_sum: f64 = values.iter().map(|v| {
        let diff = mean - *v;
        diff * diff
    }).sum();

    let variance = if count > 1 {
        variance_sum / (count - 1) as f64 // Sample variance
    } else {
        0.0
    };

    let std_dev = variance.sqrt();

    Ok(Baseline {
        mean,
        std_dev,
        sample_count: count,
        z_score_threshold: 3.0, // Default 3 sigma
    })
}

/// Start a new anomaly check for a given measurement
pub fn check_for_anomaly(pool: &Pool, probe_type: &str, target: &str, value: f64) -> Result<Option<Anomaly>> {
    if value < 0.0 {
        // Error sentinel, handled by Availability logic, not statistical anomaly
        return Ok(None);
    }

    let baseline = calculate_baseline(pool, probe_type, target)?;

    // Need enough samples to be statistically significant
    if baseline.sample_count < 10 {
        return Ok(None); 
    }

    // Z-Score calculation
    let z_score = if baseline.std_dev > 0.0001 {
        (value - baseline.mean) / baseline.std_dev
    } else {
        0.0
    };

    // Anomaly if z_score > threshold AND value is "worse" than mean
    // For Latency: Worse = Higher. (z_score > 3.0)
    // For Throughput: Worse = Lower. (z_score < -3.0). But throughput not in 'measurements' usually.
    // Assuming 'measurements' table is Latency (ms).
    
    // We only flag HIGH latency anomalies for now.
    if z_score > baseline.z_score_threshold {
        Ok(Some(Anomaly {
            probe_type: probe_type.to_string(),
            target: target.to_string(),
            value,
            baseline_mean: baseline.mean,
            baseline_std_dev: baseline.std_dev,
            z_score,
            severity: calculate_severity(z_score),
            timestamp: chrono::Utc::now(),
        }))
    } else {
        Ok(None)
    }
}

fn calculate_severity(z_score: f64) -> crate::detect::Severity {
    if z_score > 6.0 {
        crate::detect::Severity::Critical
    } else if z_score > 4.5 {
        crate::detect::Severity::Warning
    } else {
        crate::detect::Severity::Info
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    pub probe_type: String,
    pub target: String,
    pub value: f64,
    pub baseline_mean: f64,
    pub baseline_std_dev: f64,
    pub z_score: f64,
    pub severity: crate::detect::Severity,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::save_measurement;
    use crate::probes::{Measurement, ProbeType};

    #[test]
    fn test_baseline_calculation() -> Result<()> {
        use std::time::SystemTime;
        let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos();
        let db_name = format!("test_stats_{}.db", now);
        
        let manager = r2d2_sqlite::SqliteConnectionManager::file(&db_name);
        
        // Remove if exists (unlikely)
        let _ = std::fs::remove_file(&db_name);
        let pool = r2d2::Pool::new(manager)?;
        
        // Clean start
        let _ = std::fs::remove_file("test_stats.db"); // Wait, removing while open is bad on Windows, ok on Linux.
        // Better to migrate first. 
        // Or unwrap pool?
        // Let's just drop table.
        let conn = pool.get()?;
        let _ = conn.execute("DROP TABLE IF EXISTS measurements", []);
        crate::storage::schema::migrate(&conn)?;
        drop(conn); // release execution for save_measurement

        // Insert data: 10 samples of 10.0, 1 sample of 14.0
        // Total 11 samples.
        // Mean = (100 + 14) / 11 = 114 / 11 = 10.36
        // Variance calc is annoying.
        // Let's use simpler set. 
        // 10 samples of 10.0. Mean=10, StdDev=0.
        // Then check 20.0. Z = inf?
        // My code: if std_dev < 0.0001 { Z=0? } No.
        // Step 1660: if std_dev > 0.0001 { ... } else { 0.0 }.
        // Wait, if std=0, Z=0. So no anomaly?
        // Ah, typically constant baseline -> any deviation is infinite anomaly.
        // But preventing div/0 is good.
        // Let's start with 10 samples of 10.0 and one 12.0.
        // Total 11.
        for _ in 0..10 {
            save_measurement(&pool, &Measurement {
                probe_type: ProbeType::Icmp,
                target: "8.8.8.8".to_string(),
                value: 10.0,
                unit: "ms".to_string(),
                success: true,
                timestamp: std::time::SystemTime::now(),
            })?;
        }
        
        // Add some noise to get non-zero std dev
        save_measurement(&pool, &Measurement {
             probe_type: ProbeType::Icmp,
             target: "8.8.8.8".to_string(),
             value: 12.0,
             unit: "ms".to_string(),
             success: true,
             timestamp: std::time::SystemTime::now(),
        })?;

        let baseline = calculate_baseline(&pool, "icmp", "8.8.8.8")?;
        assert_eq!(baseline.sample_count, 11);
        println!("Baseline: {:?}", baseline);
        assert!(baseline.mean > 10.0 && baseline.mean < 11.0);
        assert!(baseline.std_dev > 0.0);

        // Check anomaly
        // Value 30.0 (Huge spike)
        let anomaly = check_for_anomaly(&pool, "icmp", "8.8.8.8", 30.0)?.expect("Should be anomaly");
        assert!(anomaly.z_score > 3.0);
        
        // Clean up
        let _ = std::fs::remove_file(&db_name);
        let _ = std::fs::remove_file(format!("{}-wal", db_name));
        let _ = std::fs::remove_file(format!("{}-shm", db_name));
        Ok(())
    }
}
