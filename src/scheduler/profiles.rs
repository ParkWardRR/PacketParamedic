//! Default schedule profiles and user overrides.

/// Standard schedule profile structure.
pub struct DefaultSchedule {
    pub name: String,
    pub cron_expr: String,
    pub test_type: String,
    pub enabled: bool,
}

pub enum Profile {
    Minimal,
    Standard,
    Aggressive,
}

impl Profile {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "minimal" => Some(Self::Minimal),
            "standard" | "default" => Some(Self::Standard),
            "aggressive" => Some(Self::Aggressive),
            _ => None,
        }
    }
}

pub fn get_profile_schedules(profile: Profile) -> Vec<DefaultSchedule> {
    match profile {
        Profile::Minimal => vec![
            DefaultSchedule {
                name: "gateway-ping".to_string(),
                cron_expr: "* * * * *".to_string(),
                test_type: "icmp-gateway".to_string(),
                enabled: true,
            },
            DefaultSchedule {
                name: "daily-speed-test".to_string(),
                cron_expr: "0 3 * * *".to_string(),
                test_type: "speed-test-light".to_string(),
                enabled: true,
            },
        ],
        Profile::Standard => defaults(),
        Profile::Aggressive => {
            let mut scheds = defaults();
            // Add more aggressive checks
            scheds.push(DefaultSchedule {
                name: "aggressive-jitter-check".to_string(),
                cron_expr: "*/30 * * * * *".to_string(), // every 30s
                test_type: "icmp:8.8.8.8".to_string(),
                enabled: true,
            });
            scheds.push(DefaultSchedule {
                name: "hourly-speed-test".to_string(),
                cron_expr: "0 * * * *".to_string(),
                test_type: "speed-test-light".to_string(),
                enabled: true, 
            });
            scheds
        }
    }
}

/// Return the default out-of-box schedules (Standard profile).
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
