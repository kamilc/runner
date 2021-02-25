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

        // todo: 1. bring back disk minor and major as it was
        //       2. do the stdout and stderr redirection
        //       3. store id and pid in the processed hashmap

        Command::new(&request.command)
            .args(&request.arguments)
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
}
