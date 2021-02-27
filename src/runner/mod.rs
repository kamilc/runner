mod cgroups;

#[macro_use]
mod service;

use anyhow::{anyhow, Context, Result};
use cgroups::create_cgroups;
use controlgroup;
use futures::stream::Stream;
use log::warn;
use service::{
    log_request,
    log_response::{log_error, LogError},
    run_request::Disk,
    run_response::{run_error, RunError},
    status_response::{status_error, status_result, Status, StatusError, StatusResult},
    stop_response::{stop_error, StopError},
    InternalError, LogRequest, RunRequest, StatusRequest, StopRequest, TaskError,
};
use std::collections::HashMap;
use std::default::Default;
use std::fs::File;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::pin::Pin;
use std::process::Command;
use std::process::ExitStatus;
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::sync::RwLock;
use std::thread;
use std::time::Duration;
use sysinfo::{ProcessExt, SystemExt};
use uuid::Uuid;

type ProcessMap = Arc<RwLock<HashMap<String, (u32, Option<ExitStatus>)>>>;

#[derive(Default)]
pub struct Runner {
    /// an internal map from UUID to ExitStatus
    processes: ProcessMap,

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
                let pid = controlgroup::Pid::from(&child);

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
                let sys_pid = child.id();

                thread::Builder::new()
                    .spawn(move || {
                        insert_process(processes.clone(), &process_id, sys_pid);

                        if let Ok(exit_status) = child.wait() {
                            update_process(processes.clone(), &process_id, sys_pid, exit_status);
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
        if let Some(pid) = self.pid_for_process(&request.id) {
            let (send, recv) = channel();
            let system = sysinfo::System::new();

            if let None = system.get_process(pid as i32) {
                return task_error!(
                    "Process already stopped",
                    stop_error::Error::ProcessAlreadyStoppedError
                );
            }

            thread::spawn(move || loop {
                if let Some(process) = system.get_process(pid as i32) {
                    process.kill(sysinfo::Signal::Term);
                } else {
                    send.send(()).unwrap();
                    break;
                }
                thread::sleep(Duration::from_millis(200));
            });

            if let Err(_) = recv.recv_timeout(Duration::from_millis(5000)) {
                let system = sysinfo::System::new();

                if let Some(process) = system.get_process(pid as i32) {
                    if !process.kill(sysinfo::Signal::Kill) {
                        return task_error!(
                            "Couldn't kill a process",
                            stop_error::Error::CouldntStopError
                        );
                    }
                }
            }

            Ok(())
        } else {
            task_error!("Process not found", stop_error::Error::ProcessNotFoundError)
        }
    }

    pub fn status(&mut self, request: &StatusRequest) -> Result<StatusResult, StatusError> {
        let map = self.processes.read().unwrap();

        if let Some((_, maybe_status)) = map.get(&request.id) {
            match maybe_status {
                Some(status) => {
                    let result = match status.code() {
                        Some(code) => status_result::Result::ExitCode(code),
                        None => match status.signal() {
                            Some(signal) => status_result::Result::Signal(signal),
                            None => Err(anyhow!("Couldn't get exit code or the kill signal"))?,
                        },
                    };

                    Ok(StatusResult {
                        status: Status::Stopped as i32,
                        result: Some(result),
                    })
                }
                None => Ok(StatusResult {
                    status: Status::Running as i32,
                    result: None,
                }),
            }
        } else {
            task_error!(
                "Process not found",
                status_error::Error::ProcessNotFoundError
            )
        }
    }

    pub fn log(
        &mut self,
        request: &LogRequest,
    ) -> Result<Box<dyn Stream<Item = Result<String, LogError>>>, LogError> {
        let map = self.processes.read().unwrap();

        if let Some((_pid, _)) = map.get(&request.id) {
            let maybe_descriptor = log_request::Descriptor::from_i32(request.descriptor);

            match maybe_descriptor {
                Some(descriptor) => {
                    let log_path = match descriptor {
                        log_request::Descriptor::Stdout => self.stdout_path(&request.id),
                        log_request::Descriptor::Stderr => self.stderr_path(&request.id),
                    };

                    unimplemented!();
                }
                None => {
                    return internal_error!(
                      "Given descriptor is invalid. Are you using compatible version of the client?"
                    )
                }
            }
        } else {
            task_error!("Process not found", log_error::Error::ProcessNotFoundError)
        }
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
            return task_error!("Command name empty", run_error::Error::NameEmptyError);
        }

        if let Some(Disk::MaxDisk(max)) = request.disk {
            if max > 1000 {
                return task_error!(
                    "Max disk weight given greater than 1000 which is invalid",
                    run_error::Error::InvalidMaxDisk
                );
            }
        }

        for arg in &request.arguments {
            if arg.trim().is_empty() {
                return task_error!(
                    "One of arguments found empty",
                    run_error::Error::ArgEmptyError
                );
            }
        }

        Ok(())
    }

    fn pid_for_process(&self, id: &String) -> Option<u32> {
        let processes = self.processes.read().unwrap();

        if let Some((pid, _)) = (*processes).get(id) {
            Some(pid.clone())
        } else {
            None
        }
    }
}

fn insert_process(processes: ProcessMap, id: &str, pid: u32) {
    // todo: think about error handling here as theoretically if the lock is poisoned
    // we're gonna have unwrap panic here
    let mut map = processes.write().unwrap();

    (*map).insert(id.to_string(), (pid, None));
}

fn update_process(processes: ProcessMap, id: &str, pid: u32, exit_code: ExitStatus) {
    // todo: think about error handling here as theoretically if the lock is poisoned
    // we're gonna have unwrap panic here
    let mut map = processes.write().unwrap();

    (*map).insert(id.to_string(), (pid, Some(exit_code)));
}
