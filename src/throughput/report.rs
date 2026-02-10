//! Throughput result formatting and storage.

use super::ThroughputResult;

/// Format a throughput result as a human-readable summary.
pub fn format_summary(result: &ThroughputResult) -> String {
    let speed = if result.throughput_mbps >= 1000.0 {
        format!("{:.2} Gbps", result.throughput_mbps / 1000.0)
    } else {
        format!("{:.1} Mbps", result.throughput_mbps)
    };

    let mut summary = format!(
        "{} {} test: {} ({} stream{}, {:.0}s, engine: {})",
        result.mode,
        result.direction,
        speed,
        result.streams,
        if result.streams == 1 { "" } else { "s" },
        result.duration_secs,
        result.engine,
    );

    if let Some(jitter) = result.jitter_ms {
        summary.push_str(&format!(", jitter: {:.2}ms", jitter));
    }
    if let Some(loss) = result.loss_percent {
        summary.push_str(&format!(", loss: {:.2}%", loss));
    }

    summary
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_summary_gbps() {
        let result = ThroughputResult {
            mode: "lan".to_string(),
            direction: "download".to_string(),
            throughput_mbps: 9412.0,
            jitter_ms: Some(0.05),
            loss_percent: Some(0.01),
            streams: 4,
            duration_secs: 30.0,
            link_speed_mbps: Some(10000),
            engine: "iperf3".to_string(),
        };
        let summary = format_summary(&result);
        assert!(summary.contains("9.41 Gbps"));
        assert!(summary.contains("4 streams"));
        assert!(summary.contains("iperf3"));
    }

    #[test]
    fn test_format_summary_mbps() {
        let result = ThroughputResult {
            mode: "wan".to_string(),
            direction: "upload".to_string(),
            throughput_mbps: 245.3,
            jitter_ms: None,
            loss_percent: None,
            streams: 1,
            duration_secs: 10.0,
            link_speed_mbps: Some(1000),
            engine: "native".to_string(),
        };
        let summary = format_summary(&result);
        assert!(summary.contains("245.3 Mbps"));
        assert!(summary.contains("1 stream,"));
    }
}
