use std::collections::HashMap;
use std::process::ExitStatus;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub enum ProcessStatus {
    Running,
    Stopped(ExitStatus),
}

/// Atomically reference counted RwLock for a hashmap of processes states
pub type ProcessMap = Arc<RwLock<HashMap<Uuid, (u32, ProcessStatus)>>>;
