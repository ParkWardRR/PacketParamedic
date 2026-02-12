//! Default schedule profiles and user overrides.

/// Standard schedule profile structure.
pub struct DefaultSchedule {
    pub name: String,
    pub cron_expr: String,
    pub test_type: String,
    pub enabled: bool,
}

/// Return the default out-of-box schedules.
pub fn defaults() -> Vec<DefaultSchedule> {
    vec![
        DefaultSchedule {
            name: "gateway-ping".to_string(),
            cron_expr: "* * * * *".to_string(), // every minute
            test_type: "icmp-gateway".to_string(),
            enabled: true,
        },
        DefaultSchedule {
            name: "dns-check".to_string(),
            cron_expr: "*/5 * * * *".to_string(), // every 5 minutes
            test_type: "dns-resolver".to_string(),
            enabled: true,
        },
        DefaultSchedule {
            name: "http-check".to_string(),
            cron_expr: "*/5 * * * *".to_string(), // every 5 minutes
            test_type: "http-reachability".to_string(),
            enabled: true,
        },
        DefaultSchedule {
            name: "daily-speed-test".to_string(),
            cron_expr: "0 3 * * *".to_string(), // 3am daily
            test_type: "speed-test-light".to_string(),
            enabled: true,
        },
        DefaultSchedule {
            name: "weekly-blame-check".to_string(),
            cron_expr: "0 4 * * 0".to_string(), // 4am Sunday
            test_type: "blame-check".to_string(),
            enabled: true,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults_have_five_schedules() {
        assert_eq!(defaults().len(), 5);
    }

    #[test]
    fn test_all_defaults_enabled() {
        assert!(defaults().iter().all(|s| s.enabled));
    }
}
