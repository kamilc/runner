mod cgroups;
mod service;

use anyhow::{anyhow, Context, Error, Result};
use cgroups::create_cgroups;
use controlgroup::Pid;
use log::warn;
use service::{
    log_response::LogError,
    run_response::RunError,
    status_response::{StatusError, StatusResult},
    stop_response::StopError,
    LogRequest, RunRequest, StatusRequest, StopRequest,
};
use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;
use std::process::Command;
use std::process::ExitStatus;
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::sync::RwLock;
use std::thread;
use uuid::Uuid;

// todo: implement std::iter::Iterator for this stream
// with Item=LogResponse
pub struct LogStream;

pub struct Runner {
    /// an internal map from UUID to ExitStatus
    processes: Arc<RwLock<HashMap<String, Option<ExitStatus>>>>,

    /// where to keep process logs
    log_dir: PathBuf,
}

impl Runner {
    pub fn run(&mut self, request: &RunRequest) -> Result<String, RunError> {
        let id = Uuid::new_v4().to_string();
        let mut cgroups = create_cgroups(request, &id)?;
        let stdout = File::open(self.stdout_path(&id))?;
        let stderr = File::open(self.stderr_path(&id))?;

        Command::new(&request.command)
            .args(&request.arguments)
            .stdout(stdout)
            .stderr(stderr)
            .spawn()
            .map(|mut child| {
                cgroups
                    .add_task(Pid::from(&child))
                    .context("Couldn't add new process to the new Linux control group")
                    .map_err(|err| match &child.kill() {
                        Ok(_) => err,
                        Err(kerr) => anyhow!(
                            "Couldn't kill the process after failing to apply a control group: {}",
                            kerr
                        ),
                    })?;

                let process_id = id.clone();
                let processes = self.processes.clone();

                thread::Builder::new()
                    .spawn(move || {
                        insert_process(processes.clone(), &process_id);

                        if let Ok(exit_status) = child.wait() {
                            update_process(processes.clone(), &process_id, exit_status);
                        } else {
                            warn!("Couldn't get the exit code for {}", process_id);
                        }
                    })
                    .context("OS refused to start a thread to watch for process exit status")?;

                Ok(id)
            })
            .context("Couldn't spawn the process as specified")?
    }

    pub fn stop(&mut self, _request: &StopRequest) -> Result<(), StopError> {
        unimplemented!();
    }

    pub fn status(&mut self, _request: &StatusRequest) -> Result<StatusResult, StatusError> {
        unimplemented!();
    }

    pub fn log(&mut self, _request: &LogRequest) -> Result<LogStream, LogError> {
        unimplemented!();
    }

    fn stdout_path(&self, id: &str) -> &str {
        unimplemented!();
    }

    fn stderr_path(&self, id: &str) -> &str {
        unimplemented!();
    }
}

fn insert_process(processes: Arc<RwLock<HashMap<String, Option<ExitStatus>>>>, id: &str) {
    // todo: think about error handling here as theoretically if the lock is poisoned
    // we're gonna have unwrap panic here
    let mut map = processes.write().unwrap();

    (*map).insert(id.to_string(), None);
}

fn update_process(
    processes: Arc<RwLock<HashMap<String, Option<ExitStatus>>>>,
    id: &str,
    exit_code: ExitStatus,
) {
    // todo: think about error handling here as theoretically if the lock is poisoned
    // we're gonna have unwrap panic here
    let mut map = processes.write().unwrap();

    (*map).insert(id.to_string(), Some(exit_code));
}
