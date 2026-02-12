use crate::probes::{self, Probe};
use crate::scheduler::Scheduler;
use crate::storage::save_measurement;
use crate::system::network; // Import the network module
use std::time::Duration;
use tracing::{error, info, warn};

/// Main scheduler execution loop.
/// Spawns a background task that polls for due schedules every 10 seconds.
pub async fn run_scheduler_loop(scheduler: Scheduler) {
    info!("Scheduler engine started");

    let mut interval = tokio::time::interval(Duration::from_secs(10));

    loop {
        interval.tick().await;

        match scheduler.check_due_tasks().await {
            Ok(tasks) => {
                for (name, full_test_string) in tasks {
                    info!(schedule=%name, "Task due");

                    // clone for async task
                    let scheduler = scheduler.clone();
                    let name = name.clone();
                    let full_test_string = full_test_string.clone();

                    tokio::spawn(async move {
                        // Mark as run BEFORE execution to prevent double-scheduling
                        if let Err(e) = scheduler.update_last_run(&name).await {
                            error!(schedule=%name, "Failed to update last_run: {}", e);
                            return;
                        }

                        // Resolve Aliases first
                        let resolved_spec = match full_test_string.as_str() {
                            "icmp-gateway" => match network::get_default_gateway() {
                                Ok(gw) => format!("icmp:{}", gw),
                                Err(e) => {
                                    warn!(schedule=%name, "Failed to resolve gateway: {}. Fallback to 192.168.1.1", e);
                                    "icmp:192.168.1.1".to_string()
                                }
                            },
                            "dns-check" | "dns-resolver" => "dns:1.1.1.1".to_string(),
                            "http-check" | "http-reachability" => "http:google.com".to_string(),
                            "speed-test-light" => "speed:wan".to_string(), // new speed alias
                            "blame-check" => "blame:full".to_string(),     // explicit blame alias
                            other => other.to_string(),
                        };

                        // Parse "type:target" e.g. "icmp:8.8.8.8"
                        let parts: Vec<&str> = resolved_spec.splitn(2, ':').collect();
                        if parts.len() != 2 {
                            warn!(schedule=%name, spec=%resolved_spec, "Invalid test spec. Expected 'type:target' (or known alias)");
                            return;
                        }
                        let probe_kind = parts[0];
                        let target = parts[1];

                        let timeout = Duration::from_secs(5);

                        let result = match probe_kind {
                            "icmp" => {
                                let p = probes::icmp::IcmpProbe;
                                p.run(target, timeout).await
                            }
                            "http" => {
                                let p = probes::http::HttpProbe::default();
                                p.run(target, timeout).await
                            }
                            "dns" => {
                                let p = probes::dns::DnsProbe::default();
                                p.run(target, timeout).await
                            }
                            "tcp" => {
                                let p = probes::tcp::TcpProbe;
                                p.run(target, timeout).await
                            }
                            "blame" => {
                                // Blame check is special: it reads from DB and writes to DB.
                                match crate::analysis::runner::perform_blame_analysis(
                                    scheduler.get_pool(),
                                )
                                .await
                                {
                                    Ok(_) => {
                                        info!(schedule=%name, "Blame analysis complete");
                                        return; // Success
                                    }
                                    Err(e) => Err(e),
                                }
                            }
                            "speed" => {
                                // "speed:wan" or "speed:lan"
                                let mode = if target == "lan" { "lan" } else { "wan" };
                                
                                info!(schedule=%name, "Waiting for bandwidth permit...");
                                let sem = scheduler.get_bandwidth_permit();
                                let _permit = match sem.acquire().await {
                                    Ok(p) => {
                                        info!(schedule=%name, "Bandwidth permit acquired");
                                        p
                                    },
                                    Err(e) => {
                                        error!(schedule=%name, "Failed to acquire bandwidth permit: {}", e);
                                        return;
                                    }
                                };

                                // Default params for scheduled test: 10s, 1 stream (lightweight)
                                match crate::throughput::run_test(mode, None, "10s", 1).await {
                                    Ok(_) => {
                                        info!(schedule=%name, mode=%mode, "Speed test complete");
                                        return; // Success
                                    }
                                    Err(e) => Err(e),
                                }
                            }
                            _ => {
                                warn!(schedule=%name, kind=%probe_kind, "Unknown probe type");
                                return;
                            }
                        };

                        match result {
                            Ok(m) => {
                                info!(schedule=%name, kind=%probe_kind, target=%target, value=%m.value, success=%m.success, "Probe finished");
                                if let Err(e) = save_measurement(scheduler.get_pool(), &m) {
                                    error!(schedule=%name, "Failed to save measurement: {}", e);
                                }
                            }
                            Err(e) => {
                                error!(schedule=%name, "Probe failed: {}", e);
                            }
                        }
                    });
                }
            }
            Err(e) => {
                error!("Failed to check due tasks: {}", e);
            }
        }
    }
}
