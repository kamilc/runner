use std::collections::HashMap;
use std::process::ExitStatus;
use std::sync::Arc;
use std::sync::RwLock;

pub type ProcessMap = Arc<RwLock<HashMap<String, (u32, Option<ExitStatus>)>>>;

pub fn insert_process(processes: ProcessMap, id: &str, pid: u32) {
    let mut map = processes.write().unwrap();

    (*map).insert(id.to_string(), (pid, None));
}

pub fn update_process(processes: ProcessMap, id: &str, pid: u32, exit_code: ExitStatus) {
    let mut map = processes.write().unwrap();

    (*map).insert(id.to_string(), (pid, Some(exit_code)));
}
