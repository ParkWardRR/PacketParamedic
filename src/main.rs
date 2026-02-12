use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "packetparamedic",
    about = "Appliance-grade network diagnostics for Raspberry Pi 5",
    version,
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the daemon (API server + scheduler + probes)
    Serve {
        /// Bind address
        #[arg(long, default_value = "0.0.0.0:8080")]
        bind: String,
    },

    /// Run hardware self-test (Pi 5 board, Wi-Fi, 10GbE NIC, thermals)
    SelfTest {
        /// JSON output for machine parsing
        #[arg(long)]
        json: bool,
    },

    /// Run a blame check ("Is it me or my ISP?")
    BlameCheck,

    /// Run a throughput / speed test
    SpeedTest {
        /// Test mode: lan or wan (deprecated if provider set)
        #[arg(long, default_value = "wan")]
        mode: String,

        /// Provider to use: ookla, ndt7, fast (overrides mode/peer)
        #[arg(long)]
        provider: Option<String>,

        /// Peer address for LAN tests
        #[arg(long)]
        peer: Option<String>,

        /// Test duration
        #[arg(long, default_value = "30s")]
        duration: String,

        /// Number of parallel TCP streams
        #[arg(long, default_value = "1")]
        streams: u32,
    },

    /// Run a trace (MTR) to a target
    Trace {
        /// Target IP or hostname
        #[arg(long, default_value = "8.8.8.8")]
        target: String,
    },

    /// Advanced Diagnostics (Phase 13)
    Diagnostics {
        #[command(subcommand)]
        cmd: DiagnosticCommand,
    },

    /// Manage scheduled tests
    Schedule {
        #[command(subcommand)]
        action: ScheduleAction,
    },

    /// Export a support/evidence bundle
    ExportBundle {
        /// Output file path
        #[arg(long, default_value = "bundle.zip")]
        output: String,
    },
}

#[derive(Subcommand)]
enum DiagnosticCommand {
    /// Measure Bufferbloat (Latency Under Load)
    Bufferbloat {
        /// Target to ping for latency measurement
        #[arg(long, default_value = "8.8.8.8")]
        target: String,
    },

    /// Manage scheduled tests
    Schedule {
        #[command(subcommand)]
        action: ScheduleAction,
    },

    /// Export a support/evidence bundle
    ExportBundle {
        /// Output file path
        #[arg(long, default_value = "bundle.zip")]
        output: String,
    },
}

#[derive(Subcommand)]
enum ScheduleAction {
    /// List all schedules
    List,

    /// Add a new schedule
    Add {
        /// Schedule name
        #[arg(long)]
        name: String,

        /// Cron expression (5-field)
        #[arg(long)]
        cron: String,

        /// Test type to run
        #[arg(long)]
        test: String,
    },

    /// Remove a schedule
    Remove {
        /// Schedule name
        #[arg(long)]
        name: String,
    },

    /// Preview what will run in the next N hours
    DryRun {
        /// Hours to preview
        #[arg(long, default_value = "24")]
        hours: u64,
    },

    /// Apply a standardized schedule profile
    ApplyProfile {
        /// Profile name (minimal, standard, aggressive)
        #[arg(long)]
        profile: String,
        
        /// Force replace all existing schedules
        #[arg(long)]
        force: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { bind } => {
            tracing::info!(%bind, "Starting PacketParamedic daemon");
            packetparamedic::serve(&bind, "data/packetparamedic.db").await?;
        }
        Commands::SelfTest { json } => {
            tracing::info!("Running hardware self-test");
            let report = packetparamedic::selftest::run().await?;
            if json {
                let json_output = serde_json::to_string_pretty(&report)?;
                println!("{}", json_output);
            } else {
                println!("\nPacketParamedic Hardware Self-Test");
                println!("{:<25} | {:<10} | Details", "Component", "Status");
                println!("{:-<25}-|-{:-<10}-|-{:-<40}", "", "", "");
                for res in &report.results {
                    let status_str = match res.status {
                        packetparamedic::selftest::TestStatus::Pass => "PASS",
                        packetparamedic::selftest::TestStatus::Fail => "FAIL",
                        packetparamedic::selftest::TestStatus::Warning => "WARN",
                        packetparamedic::selftest::TestStatus::Skipped => "SKIP",
                    };
                    println!(
                        "{:<25} | {:<10} | {}",
                        res.component, status_str, res.details
                    );
                    if let Some(rem) = &res.remediation {
                        println!("{:<25} | {:<10} |   -> Recommendation: {}", "", "", rem);
                    }
                }
                println!("\n=== Persona Readiness ===");
                for (persona, compatible) in report.compatibility {
                    let check = if compatible {
                        "✅ READY"
                    } else {
                        "❌ NOT READY"
                    };
                    println!("{:<25} : {}", persona, check);
                }
                println!("(See BUYERS_GUIDE.md for details on requirements)");
                println!();
            }
        }
        Commands::BlameCheck => {
            tracing::info!("Running blame check");
            // CLI immediate mode
            let report = packetparamedic::probes::run_blame_check().await?;

            println!("\n=== PacketParamedic Diagnostic Report ===");
            println!("Verdict:    {}", report.verdict);
            println!("Confidence: {}%", report.confidence);
            println!("\nEvidence:");
            for detail in report.details {
                println!(" - {}", detail);
            }
            println!("=========================================\n");
        }
        Commands::SpeedTest {
            mode,
            provider,
            peer,
            duration,
            streams,
        } => {
            if let Some(prov_id) = provider {
                tracing::info!(%prov_id, "Running provider speed test");
                // Dispatch to provider framework
                // Note: Real implementation would map strings to providers dynamically.
                // For MVP CLI, we just hardcode the dispatch here or print support.
                match prov_id.as_str() {
                    "ookla" | "ookla-cli" => {
                        let p = packetparamedic::throughput::provider::ookla::OoklaProvider;
                        use packetparamedic::throughput::provider::SpeedTestProvider;
                        if p.is_available() {
                            let res = p.run(packetparamedic::throughput::provider::SpeedTestRequest {
                                timeout: std::time::Duration::from_secs(30),
                                prefer_ipv6: false,
                                server_hint: None,
                            })?;
                            println!("{}", serde_json::to_string_pretty(&res)?);
                        } else {
                            anyhow::bail!("Ookla CLI not found. {}", p.meta().install_hint);
                        }
                    },
                    "ndt7" => {
                         let p = packetparamedic::throughput::provider::ndt7::Ndt7Provider;
                         use packetparamedic::throughput::provider::SpeedTestProvider;
                         if p.is_available() {
                            let res = p.run(packetparamedic::throughput::provider::SpeedTestRequest {
                                timeout: std::time::Duration::from_secs(30),
                                prefer_ipv6: false,
                                server_hint: None,
                            })?;
                            println!("{}", serde_json::to_string_pretty(&res)?);
                         } else {
                            anyhow::bail!("NDT7 Client not found. {}", p.meta().install_hint);
                         }
                    },
                     "fast" | "fast-cli" => {
                         let p = packetparamedic::throughput::provider::fast::FastProvider;
                         use packetparamedic::throughput::provider::SpeedTestProvider;
                         if p.is_available() {
                            let res = p.run(packetparamedic::throughput::provider::SpeedTestRequest {
                                timeout: std::time::Duration::from_secs(30),
                                prefer_ipv6: false,
                                server_hint: None,
                            })?;
                            println!("{}", serde_json::to_string_pretty(&res)?);
                         } else {
                            anyhow::bail!("Fast CLI not found. {}", p.meta().install_hint);
                         }
                    },
                    _ => anyhow::bail!("Unknown provider: {}", prov_id),
                }
            } else {
                tracing::info!(%mode, ?peer, %duration, %streams, "Running iperf3 speed test");
                packetparamedic::throughput::run_test(&mode, peer.as_deref(), &duration, streams)
                    .await?;
            }
        }
        Commands::Trace { target } => {
            tracing::info!(%target, "Running MTR trace");
            let report = packetparamedic::probes::trace::run_trace(&target)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Commands::Diagnostics { cmd } => {
            match cmd {
                DiagnosticCommand::Bufferbloat { target } => {
                    println!("Running Bufferbloat Analysis (Target: {})...", target);
                    let result = packetparamedic::analysis::qos::run_qos_test(&target).await?;
                    println!("\n--- Bufferbloat Grade: {} ---", result.grade);
                    println!("Baseline RTT: {:.2} ms", result.baseline_rtt_ms);
                    println!("Loaded RTT:   {:.2} ms (+{:.2} ms)", result.loaded_rtt_ms, result.bufferbloat_ms);
                    
                    if result.grade == 'D' || result.grade == 'F' {
                        println!("⚠️  High Bufferbloat detected! Your router may need AQM/SQM enabled.");
                    }
                }
            }
        }
        Commands::Schedule { action } => {
            let pool = packetparamedic::storage::open_pool("data/packetparamedic.db")?;
            let scheduler = packetparamedic::scheduler::Scheduler::new(pool);

            match action {
                ScheduleAction::List => {
                    let list = scheduler.list().await?;
                    if list.is_empty() {
                        println!("No schedules found.");
                    } else {
                        println!("{:<20} | {:<15} | {:<10} | Enabled", "Name", "Cron", "Test");
                        println!("{:-<20}-|-{:-<15}-|-{:-<10}-|-{:-<7}", "", "", "", "");
                        for (name, cron, test, enabled) in list {
                            println!("{:<20} | {:<15} | {:<10} | {}", name, cron, test, enabled);
                        }
                    }
                }
                ScheduleAction::Add { name, cron, test } => {
                    scheduler.add_schedule(&name, &cron, &test).await?;
                    println!("Schedule '{}' added.", name);
                }
                ScheduleAction::Remove { name } => {
                    scheduler.remove(&name).await?;
                    println!("Schedule '{}' removed.", name);
                }
                ScheduleAction::DryRun { hours } => {
                    let preview = scheduler.preview_next_runs(hours).await?;
                    if preview.is_empty() {
                        println!("No runs scheduled in next {} hours.", hours);
                    } else {
                        println!("Upcoming runs (next {} hours):", hours);
                        for (time, name, test) in preview {
                            println!("{} : {} ({})", time, name, test);
                        }
                    }
                }
                ScheduleAction::ApplyProfile { profile, force } => {
                    use packetparamedic::scheduler::profiles::Profile;
                    if let Some(p) = Profile::from_str(&profile) {
                         if !force {
                             println!("WARNING: This will DELETE all existing schedules and apply the '{}' profile.", profile);
                             println!("Pass --force to confirm.");
                             return Ok(());
                         }
                         
                         // Delete all
                         let list = scheduler.list().await?;
                         for (n, _, _, _) in list {
                             scheduler.remove(&n).await?;
                         }
                         
                         // Apply profile
                         let scheds = packetparamedic::scheduler::profiles::get_profile_schedules(p);
                         for s in scheds {
                             scheduler.add_schedule(&s.name, &s.cron_expr, &s.test_type).await?;
                             if !s.enabled {
                                 // toggle disable if logic existed, but add_schedule default enables.
                                 // For now we assume enabled.
                             }
                         }
                         println!("Profile '{}' applied successfully.", profile);
                    } else {
                        anyhow::bail!("Unknown profile: {}. Options: minimal, standard, aggressive", profile);
                    }
                }
            }
        }
        Commands::ExportBundle { output } => {
            tracing::info!(%output, "Exporting support bundle");
            packetparamedic::evidence::export_bundle(&output).await?;
        }
    }

    Ok(())
}
