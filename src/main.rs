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
        /// Test mode: lan or wan
        #[arg(long, default_value = "wan")]
        mode: String,

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
            peer,
            duration,
            streams,
        } => {
            tracing::info!(%mode, ?peer, %duration, %streams, "Running speed test");
            packetparamedic::throughput::run_test(&mode, peer.as_deref(), &duration, streams)
                .await?;
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
            }
        }
        Commands::ExportBundle { output } => {
            tracing::info!(%output, "Exporting support bundle");
            packetparamedic::evidence::export_bundle(&output).await?;
        }
    }

    Ok(())
}
