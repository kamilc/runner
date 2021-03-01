use std::collections::HashMap;
use std::process::ExitStatus;
use std::sync::Arc;
use std::sync::RwLock;

/// Atomically reference counted RwLock for a hashmap of processes states
pub type ProcessMap = Arc<RwLock<HashMap<String, (u32, Option<ExitStatus>)>>>;
