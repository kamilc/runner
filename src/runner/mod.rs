mod service;

use anyhow::{Context, Error, Result};
use controlgroup::{
    v1::{Builder, UnifiedRepr},
    Pid,
};
use service::{
    log_response, run_request, run_response, status_response, stop_response, LogRequest,
    RunRequest, StatusRequest, StopRequest,
};
use std::boxed::Box;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::RwLock;
use std::thread;
use uuid::Uuid;

// todo: implement std::iter::Iterator for this stream
// with Item=LogResponse
pub struct LogStream;

pub struct Runner {
    /// an internal map from UUID to PID
    processes: RwLock<HashMap<String, u32>>,
}

impl Runner {
    pub fn run(&mut self, request: &RunRequest) -> Result<String, run_response::Error> {
        let id = Uuid::new_v4().to_string();
        let mut cgroups = self.create_cgroups(request, &id)?;

        // todo: 1. bring back disk minor and major as it was
        //       2. do the stdout and stderr redirection
        //       3. store id and pid in the processed hashmap

        Command::new(&request.command)
            .args(&request.arguments)
            .spawn()
            .map(|child| {
                cgroups.add_task(Pid::from(&child))?;

                Ok(child.id().to_string())
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

    fn create_cgroups(&mut self, request: &RunRequest, id: &str) -> Result<UnifiedRepr> {
        let mut builder = Builder::new(PathBuf::from(id));

        if let Some(run_request::Memory::MaxMemory(max)) = request.memory {
            builder = builder.memory().limit_in_bytes(max).done();
        }

        if let Some(run_request::Cpu::MaxCpu(max)) = request.cpu {
            builder = builder.cpu().shares(max).done();
        }

        if let Some(run_request::Disk::MaxDisk(max)) = request.disk {
            builder = builder.blkio().weight(u16::try_from(max)?).done();
        }

        Ok(builder.build()?)
    }
}
