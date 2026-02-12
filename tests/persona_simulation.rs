use packetparamedic::api::state::AppState;
use packetparamedic::scheduler::Scheduler;
use packetparamedic::storage::Pool;
use std::sync::Arc;
use tokio::time::Duration;

// Mock the AppState since we can't spin up full server in unit test easily without bind port conflicts
async fn setup_test_env() -> AppState {
    let pool = packetparamedic::storage::open_pool(":memory:").unwrap();
    let scheduler = Scheduler::new(pool.clone());
    scheduler.ensure_defaults().await.unwrap();
    AppState { pool, scheduler }
}

#[tokio::test]
async fn test_persona_simple_troubleshooting() {
    let state = setup_test_env().await;

    // 1. "Is it me or my ISP?" -> Run a trace (Phase 7)
    // We mock the trace execution by calling the logic directly if possible, or just checking the route handler availability?
    // Since we can't shell out to `mtr` reliably in test env, we'll test the scheduler logic for a "quick check"
    
    // User enables the default "gateway check" schedule
    state.scheduler.add_schedule("quick-check", "*/5 * * * * *", "icmp-gateway").await.unwrap();
    
    // Verify it's scheduled
    let list = state.scheduler.list().await.unwrap();
    assert!(list.iter().any(|(n, _, _, _)| n == "quick-check"));
    
    // Simulate finding an incident (Phase 8 prep)
    // (Future: call incident manager to list recent incidents)
}

#[tokio::test]
async fn test_persona_reliability_soak() {
    let state = setup_test_env().await;

    // 1. Schedule a heavy throughput test for 3 AM
    state.scheduler.add_schedule("nightly-soak", "0 3 * * * *", "throughput-stress").await.unwrap();
    
    // 2. Verify dry-run shows it
    let upcoming = state.scheduler.preview_next_runs(24).await.unwrap();
    assert!(upcoming.iter().any(|(_, name, _)| name == "nightly-soak"));
}

#[tokio::test]
async fn test_persona_high_performance() {
    let state = setup_test_env().await;
    
    // 1. User wants to use Ookla provider (Phase 6.1)
    // We can instantiate the provider (even if we don't run it because CLI missing)
    let provider = packetparamedic::throughput::provider::ookla::OoklaProvider;
    let meta = packetparamedic::throughput::provider::SpeedTestProvider::meta(&provider);
    
    // Verify licensing note is present (crucial for this persona)
    assert!(meta.licensing_note.unwrap().contains("Personal Non-Commercial"));
    assert_eq!(meta.id, "ookla-cli");
}
