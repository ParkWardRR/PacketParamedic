//! iperf3 process wrapper -- spawn, parse JSON output, enforce timeouts.

use anyhow::Result;
use serde::Deserialize;

/// Parsed iperf3 JSON result (subset of fields we care about).
#[derive(Debug, Deserialize)]
pub struct Iperf3Result {
    pub start: Iperf3Start,
    pub end: Iperf3End,
}

#[derive(Debug, Deserialize)]
pub struct Iperf3Start {
    pub test_start: Iperf3TestStart,
}

#[derive(Debug, Deserialize)]
pub struct Iperf3TestStart {
    pub protocol: String,
    pub num_streams: u32,
    pub duration: f64,
}

#[derive(Debug, Deserialize)]
pub struct Iperf3End {
    pub sum_sent: Iperf3Sum,
    pub sum_received: Iperf3Sum,
}

#[derive(Debug, Deserialize)]
pub struct Iperf3Sum {
    pub bits_per_second: f64,
    pub bytes: u64,
    #[serde(default)]
    pub jitter_ms: Option<f64>,
    #[serde(default)]
    pub lost_percent: Option<f64>,
}

/// Parse an iperf3 JSON output string into a structured result.
pub fn parse_output(json_str: &str) -> Result<Iperf3Result> {
    let result: Iperf3Result = serde_json::from_str(json_str)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_parse_1g_tcp_fixture() {
        let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("iperf3")
            .join("1g-tcp.json");
        if fixture_path.exists() {
            let json_str = std::fs::read_to_string(&fixture_path).unwrap();
            let result = parse_output(&json_str).unwrap();
            assert_eq!(result.start.test_start.protocol, "TCP");
            assert!(result.end.sum_received.bits_per_second > 0.0);
        }
    }

    #[test]
    fn test_parse_10g_tcp_fixture() {
        let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("iperf3")
            .join("10g-tcp.json");
        if fixture_path.exists() {
            let json_str = std::fs::read_to_string(&fixture_path).unwrap();
            let result = parse_output(&json_str).unwrap();
            assert_eq!(result.start.test_start.protocol, "TCP");
            assert!(result.end.sum_received.bits_per_second > 5_000_000_000.0);
        }
    }
}
