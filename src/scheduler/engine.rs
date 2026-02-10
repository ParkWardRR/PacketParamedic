use crate::scheduler::Scheduler;
use crate::storage::save_measurement;
use crate::probes::{self, Probe};
use std::time::Duration;
use tracing::{info, warn, error};

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

                        // Parse "type:target" e.g. "icmp:8.8.8.8"
                        // Or just "icmp" (invalid)
                        let parts: Vec<&str> = full_test_string.splitn(2, ':').collect();
                        if parts.len() != 2 {
                             warn!(schedule=%name, spec=%full_test_string, "Invalid test spec. Expected 'type:target'");
                             return;
                        }
                        let probe_kind = parts[0];
                        let target = parts[1];

                        let timeout = Duration::from_secs(5);
                        
                        let result = match probe_kind {
                            "icmp" => {
                                let p = probes::icmp::IcmpProbe;
                                p.run(target, timeout).await
                            },
                             "http" => {
                                let p = probes::http::HttpProbe::default();
                                p.run(target, timeout).await
                            },
                            "dns" => {
                                let p = probes::dns::DnsProbe::default();
                                p.run(target, timeout).await
                            },
                            "tcp" => {
                                let p = probes::tcp::TcpProbe;
                                p.run(target, timeout).await
                            },
                             "blame" => {
                                 // Blame check is special: it reads from DB and writes to DB.
                                 // Target field is ignored (or could specify model?)
                                 match crate::analysis::runner::perform_blame_analysis(scheduler.get_pool()).await {
                                     Ok(_) => {
                                         info!(schedule=%name, "Blame analysis complete");
                                         return; // Success, return early as we don't have a measurement to save
                                     }
                                     Err(e) => {
                                         // Create a dummy failure measurement or just log error?
                                         // Let's create a dummy so we log failure
                                          Err(e)
                                     }
                                 }
                            },
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
                            },
                            Err(e) => {
                                error!(schedule=%name, "Probe failed: {}", e);
                            }
                        }
                    });
                }
            },
            Err(e) => {
                error!("Failed to check due tasks: {}", e);
            }
        }
    }
}
