mod service;

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

impl Runner {
    // todo: implement me
    pub fn run(&mut self, request: &RunRequest) -> Result<String, run_response::Error> {
        let id = Uuid::new_v4().to_string();
        let cgroup = self
            .create_cgroup_for(request, id)
            .context("Couldn't create a control group")?;
        let mut command = self.build_command(request);

        command.spawn().map(|child| {
            let pid = child.id();
            cgroup.add_task((pid as u64).into())?;

            Ok(pid.to_string())
        })?
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
