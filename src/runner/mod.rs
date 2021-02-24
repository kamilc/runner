pub mod service {
    tonic::include_proto!("service");
}

use anyhow::{Context, Error, Result};
use cgroups_rs::cgroup_builder::CgroupBuilder;
use cgroups_rs::{Cgroup, Hierarchy};
use service::{
    log_response, run_response, status_response, stop_response, LogRequest, RunRequest,
    StatusRequest, StopRequest,
};
use std::boxed::Box;
use std::collections::HashMap;
use std::process::{Child, Command};
use std::sync::RwLock;
use std::thread;
use uuid::Uuid;

// todo: implement std::iter::Iterator for this stream
// with Item=LogResponse
pub struct LogStream;

pub struct Runner {
    // an internal map from UUID to PID
    processes: RwLock<HashMap<String, u32>>,
    cgroups_hier: Box<dyn Hierarchy>,
}

impl std::fmt::Display for run_response::Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "error")
    }
}

macro_rules! impl_from_error {
    ($to:path, $($from:path), +) => {
        $(
            impl std::convert::From<$from> for $to {
                fn from(error: $from) -> $to {
                    run_response::Error {
                        description: error.to_string(),
                        errors: Some(run_response::error::Errors::GeneralError(1)),
                    }
                }
            }
        )+
    };
}

impl_from_error!(run_response::Error, anyhow::Error, cgroups_rs::error::Error);

impl Runner {
    // todo: implement me
    pub fn run(&mut self, request: &RunRequest) -> Result<String, run_response::Error> {
        let id = Uuid::new_v4().to_string();
        let cgroup = self
            .create_cgroup_for(request, id)
            .context("Couldn't create a control group")?;
        let mut command = self.build_command(request);

        match command.spawn() {
            Ok(child) => {
                let pid = child.id();
                cgroup.add_task((pid as u64).into())?;

                Ok(pid.to_string())
            }
            Err(_err) => Err(run_response::Error {
                description: "".to_string(),
                errors: Some(run_response::error::Errors::GeneralError(0)),
            }),
        }
    }

    pub fn stop(&mut self, _request: &StopRequest) -> Result<(), stop_response::Error> {
        unimplemented!();
    }

    pub fn status(
        &mut self,
        _request: &StatusRequest,
    ) -> Result<status_response::StatusResult, status_response::Error> {
        unimplemented!();
    }

    pub fn log(&mut self, _request: &LogRequest) -> Result<LogStream, log_response::Error> {
        unimplemented!();
    }

    fn create_cgroup_for(&mut self, _request: &RunRequest, _id: String) -> Result<Cgroup> {
        unimplemented!();
    }

    fn build_command(&mut self, _request: &RunRequest) -> Command {
        unimplemented!();
    }
}
