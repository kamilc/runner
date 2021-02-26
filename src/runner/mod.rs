mod cgroups;
mod service;

use anyhow::{anyhow, Context, Error, Result};
use cgroups::create_cgroups;
use controlgroup::Pid;
use log::warn;
use service::{
    log_response::LogError,
    run_request::Disk,
    run_response::{run_error, RunError},
    status_response::{StatusError, StatusResult},
    stop_response::{stop_error, StopError},
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
    processes: Arc<RwLock<HashMap<String, (Pid, Option<ExitStatus>)>>>,

    /// where to keep process logs
    log_dir: PathBuf,
}

impl Runner {
    pub fn run(&mut self, request: &RunRequest) -> Result<String, RunError> {
        self.validate_run(&request)?;

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
                let pid = Pid::from(&child);

                cgroups
                    .add_task(pid.clone())
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
                        insert_process(processes.clone(), &process_id, pid.clone());

                        if let Ok(exit_status) = child.wait() {
                            update_process(
                                processes.clone(),
                                &process_id,
                                pid.clone(),
                                exit_status,
                            );
                        } else {
                            warn!("Couldn't get the exit code for {}", process_id);
                        }
                    })
                    .context("OS refused to start a thread to watch for process exit status")?;

                Ok(id)
            })
            .context("Couldn't spawn the process as specified")?
    }

    pub fn stop(&mut self, request: &StopRequest) -> Result<(), StopError> {
        self.validate_stop(&request)?;

        unimplemented!();
    }

    pub fn status(&mut self, _request: &StatusRequest) -> Result<StatusResult, StatusError> {
        unimplemented!();
    }

    pub fn log(&mut self, _request: &LogRequest) -> Result<LogStream, LogError> {
        unimplemented!();
    }

    fn stdout_path(&self, id: &str) -> PathBuf {
        let mut path = PathBuf::new();

        path.push(&self.log_dir);
        path.push(format!("{}.stdout.txt", id));

        path
    }

    fn stderr_path(&self, id: &str) -> PathBuf {
        let mut path = PathBuf::new();

        path.push(&self.log_dir);
        path.push(format!("{}.stderr.txt", id));

        path
    }

    fn validate_run(&self, request: &RunRequest) -> Result<(), RunError> {
        if request.command.trim().is_empty() {
            return Err(RunError {
                description: "Given command name was empty".to_string(),
                errors: Some(run_error::Errors::RunError(
                    run_error::Error::NameEmptyError.into(),
                )),
            });
        }

        if let Some(Disk::MaxDisk(max)) = request.disk {
            if max > 1000 {
                return Err(RunError {
                    description: "Given disk weight was greater than 1000 which is invalid"
                        .to_string(),
                    errors: Some(run_error::Errors::RunError(
                        run_error::Error::InvalidMaxDisk.into(),
                    )),
                });
            }
        }

        for arg in &request.arguments {
            if arg.trim().is_empty() {
                return Err(RunError {
                    description: "One of arguments found empty".to_string(),
                    errors: Some(run_error::Errors::RunError(
                        run_error::Error::ArgEmptyError.into(),
                    )),
                });
            }
        }

        Ok(())
    }

    fn validate_stop(&self, request: &StopRequest) -> Result<(), StopError> {
        let processes = self.processes.read().unwrap();

        if !(*processes).contains_key(&request.id) {
            return Err(StopError {
                description: "Process not found".to_string(),
                errors: Some(stop_error::Errors::StopError(
                    stop_error::Error::ProcessNotFoundError.into(),
                )),
            });
        }

        Ok(())
    }
}

fn insert_process(
    processes: Arc<RwLock<HashMap<String, (Pid, Option<ExitStatus>)>>>,
    id: &str,
    pid: Pid,
) {
    // todo: think about error handling here as theoretically if the lock is poisoned
    // we're gonna have unwrap panic here
    let mut map = processes.write().unwrap();

    (*map).insert(id.to_string(), (pid, None));
}

fn update_process(
    processes: Arc<RwLock<HashMap<String, (Pid, Option<ExitStatus>)>>>,
    id: &str,
    pid: Pid,
    exit_code: ExitStatus,
) {
    // todo: think about error handling here as theoretically if the lock is poisoned
    // we're gonna have unwrap panic here
    let mut map = processes.write().unwrap();

    (*map).insert(id.to_string(), (pid, Some(exit_code)));
}
