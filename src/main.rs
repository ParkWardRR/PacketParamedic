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
    SelfTest,

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
            packetparamedic::serve(&bind).await?;
        }
        Commands::SelfTest => {
            tracing::info!("Running hardware self-test");
            packetparamedic::selftest::run().await?;
        }
        Commands::BlameCheck => {
            tracing::info!("Running blame check");
            packetparamedic::probes::blame_check().await?;
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
        Commands::Schedule { action } => match action {
            ScheduleAction::List => {
                packetparamedic::scheduler::list_schedules().await?;
            }
            ScheduleAction::Add { name, cron, test } => {
                packetparamedic::scheduler::add_schedule(&name, &cron, &test).await?;
            }
            ScheduleAction::Remove { name } => {
                packetparamedic::scheduler::remove_schedule(&name).await?;
            }
            ScheduleAction::DryRun { hours } => {
                packetparamedic::scheduler::dry_run(hours).await?;
            }
        },
        Commands::ExportBundle { output } => {
            tracing::info!(%output, "Exporting support bundle");
            packetparamedic::evidence::export_bundle(&output).await?;
        }
    }

    Ok(())
}
