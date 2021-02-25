mod cgroups;
mod service;

use anyhow::{anyhow, Context, Error, Result};
use cgroups::create_cgroups;
use controlgroup::Pid;
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
use std::sync::RwLock;
use uuid::Uuid;

// todo: implement std::iter::Iterator for this stream
// with Item=LogResponse
pub struct LogStream;

pub struct Runner {
    /// an internal map from UUID to PID
    processes: RwLock<HashMap<String, u32>>,
}

impl Runner {
    pub fn run(&mut self, request: &RunRequest) -> Result<String, RunError> {
        let id = Uuid::new_v4().to_string();
        let mut cgroups = create_cgroups(request, &id)?;
        let stdout = File::open(self.stdout_path(&id))?;
        let stderr = File::open(self.stderr_path(&id))?;

        // todo: 1. do the stdout and stderr redirection
        //       2. store id and pid in the processed hashmap

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

                Ok(child.id().to_string())
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

    fn log_dir(&self) -> PathBuf {
        unimplemented!();
    }

    fn stdout_path(&self, id: &str) -> &str {
        unimplemented!();
    }

    fn stderr_path(&self, id: &str) -> &str {
        unimplemented!();
    }
}
