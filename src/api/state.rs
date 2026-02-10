use crate::scheduler::Scheduler;
use crate::storage::Pool;

#[derive(Clone)]
pub struct AppState {
    pub pool: Pool,
    pub scheduler: Scheduler,
}
