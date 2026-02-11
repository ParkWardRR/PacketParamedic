use crate::analysis::model::BlameFeatures;
use anyhow::Result;
use rusqlite::{params, Connection};

/// Aggregate raw measurements from SQLite into a Feature Vector for the classifier
pub struct FeatureAggregator;

impl FeatureAggregator {
    /// Compute features from the last N minutes of measurements
    pub fn compute_features(conn: &Connection, window_minutes: i64) -> Result<BlameFeatures> {
        let mut features = BlameFeatures::default();

        // Query time window
        let window_start = format!("-{} minutes", window_minutes);

        // Helper query: Percentile & Loss for a given probe type/target
        // Note: SQLite doesn't have native PERCENTILE_CONT, so we approximate or fetch-and-sort in app.
        // For embedded, fetch-and-sort is fine for reasonable window sizes (e.g. 5 mins = 300 pings).

        // 1. Gateway RTT & Loss
        let (gw_p50, gw_p95, gw_loss) =
            Self::analyze_latency(conn, "icmp", "gateway", &window_start)?;
        features.gw_rtt_p50_ms = gw_p50;
        features.gw_rtt_p95_ms = gw_p95;
        features.gw_loss_pct = gw_loss;

        // 2. WAN RTT & Loss
        let (wan_p50, wan_p95, wan_loss) =
            Self::analyze_latency(conn, "icmp", "8.8.8.8", &window_start)?; // Using 8.8.8.8 as WAN proxy for now
        features.wan_rtt_p50_ms = wan_p50;
        features.wan_rtt_p95_ms = wan_p95;
        features.wan_loss_pct = wan_loss;

        // 3. Delta RTT
        features.delta_rtt_p50_ms = features.wan_rtt_p50_ms - features.gw_rtt_p50_ms;

        // 4. DNS Health
        let (dns_p50, dns_fail) = Self::analyze_app_metric(conn, "dns", &window_start)?;
        features.dns_ms_p50 = dns_p50;
        features.dns_fail_rate = dns_fail;

        // 5. HTTP/TCP Health
        let (_, http_fail) = Self::analyze_app_metric(conn, "http", &window_start)?;
        features.http_fail_rate = http_fail;

        let (_, tcp_fail) = Self::analyze_app_metric(conn, "tcp", &window_start)?;
        features.tcp_fail_rate = tcp_fail;

        // 6. Throughput (Latest measurement in window)
        // TODO: Query throughput_results table properly. For now, zero.
        features.wan_down_mbps = 0.0;
        features.wan_up_mbps = 0.0;

        Ok(features)
    }

    fn analyze_latency(
        conn: &Connection,
        probe_type: &str,
        target: &str,
        window_start: &str,
    ) -> Result<(f64, f64, f64)> {
        // Fetch all values in window
        let _stmt = conn.prepare(
            "SELECT value, duration_us FROM measurements 
             WHERE probe_type = ?1 
             AND (target = ?2 OR target = 'gateway') -- simplistic matching
             AND created_at >= datetime('now', ?3)
             ORDER BY value ASC",
        )?;

        // Note: target matching needs refinement in real app (e.g. resolve 'gateway' to actual IP)
        // For MVP, we assume target strings match what probes write.

        // Actually, let's just match exact target for now
        let mut stmt = conn.prepare(
            "SELECT value FROM measurements 
             WHERE probe_type = ?1 
             AND target = ?2
             AND created_at >= datetime('now', ?3)
             ORDER BY value ASC",
        )?;

        let rows = stmt.query_map(params![probe_type, target, window_start], |row| {
            row.get::<_, f64>(0)
        })?;

        let mut values: Vec<f64> = Vec::new();
        for v in rows.flatten() {
            if v >= 0.0 {
                values.push(v);
            } // -1 usually means timeout/loss
        }

        // Calculate Loss: Count total attempts vs success
        // This requires knowing total attempts.
        // If we store timeouts as -1 or NULL, we can count.
        // Let's assume measurements stores -1.0 for timeout.

        // Re-query including timeouts
        let mut stmt_all = conn.prepare(
            "SELECT value FROM measurements 
             WHERE probe_type = ?1 
             AND target = ?2
             AND created_at >= datetime('now', ?3)",
        )?;
        let all_rows = stmt_all.query_map(params![probe_type, target, window_start], |row| {
            row.get::<_, f64>(0)
        })?;

        let mut total = 0;
        let mut timeouts = 0;
        let mut valid_latencies = Vec::new();

        for v in all_rows.flatten() {
            total += 1;
            if v < 0.0 {
                timeouts += 1;
            } else {
                valid_latencies.push(v);
            }
        }

        if total == 0 {
            return Ok((0.0, 0.0, 0.0)); // No data
        }

        let loss_pct = (timeouts as f64 / total as f64) * 100.0;

        valid_latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let len = valid_latencies.len();

        let p50 = if len > 0 {
            valid_latencies[len / 2]
        } else {
            0.0
        };
        let p95 = if len > 0 {
            valid_latencies[(len as f64 * 0.95) as usize]
        } else {
            0.0
        };

        Ok((p50, p95, loss_pct))
    }

    fn analyze_app_metric(
        conn: &Connection,
        probe_type: &str,
        window_start: &str,
    ) -> Result<(f64, f64)> {
        let mut stmt = conn.prepare(
            "SELECT value FROM measurements 
             WHERE probe_type = ?1 
             AND created_at >= datetime('now', ?2)",
        )?;

        let rows = stmt.query_map(params![probe_type, window_start], |row| {
            row.get::<_, f64>(0)
        })?;

        let mut total = 0;
        let mut fails = 0;
        let mut valid = Vec::new();

        for v in rows.flatten() {
            total += 1;
            if v < 0.0 {
                // Assumption: < 0 is failure code
                fails += 1;
            } else {
                valid.push(v);
            }
        }

        if total == 0 {
            return Ok((0.0, 0.0));
        }

        let fail_rate = fails as f64 / total as f64;

        valid.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p50 = if !valid.is_empty() {
            valid[valid.len() / 2]
        } else {
            0.0
        };

        Ok((p50, fail_rate))
    }
}
