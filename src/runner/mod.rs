mod cgroups;

#[macro_use]
pub mod service;

mod log_stream;
mod process_map;

use anyhow::{anyhow, Context, Result};
use cgroups::create_cgroups;
use log::warn;
use log_stream::LogStream;
use nix::errno::Errno;
use nix::sys::signal;
use nix::unistd::Pid;
use process_map::{
    ProcessMap,
    ProcessStatus::{Running, Stopped},
};
use service::{
    log_request,
    log_response::{log_error, LogError},
    run_request::Disk,
    run_response::{run_error, RunError},
    status_response::{status_error, status_result, StatusError, StatusResult},
    stop_response::{stop_error, StopError},
    InternalError, LogRequest, RunRequest, StatusRequest, StopRequest, TaskError,
};
use std::fs::File;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use uuid::Uuid;

/// Processes runner struct. Includes processes states and allows to
/// run them, stop, get their status and the stream of logs
#[derive(Default, Clone, Debug)]
pub struct Runner {
    /// an internal map from UUID to ExitStatus
    processes: ProcessMap,

    /// where to keep process logs
    log_dir: String,

    /// The size of the buffer for streaming logs
    buffer_size: Option<usize>,
}

impl Runner {
    /// Executes given command. Allows to specify the command name, arguments and
    /// resource constraints. Returns a UUID of the process or an error.
    ///
    /// # Panics
    ///
    /// Panics if called from outside of the Tokio runtime.
    pub async fn run(&mut self, request: &RunRequest) -> Result<Uuid, RunError> {
        self.validate_run(&request)?;

        let id = Uuid::new_v4();
        let mut cgroups = create_cgroups(request, &id).context("Couldn't create a cgroup")?;
        let stdout =
            File::create(self.stdout_path(&id)).context("Couldn't open log file for STDOUT")?;
        let stderr =
            File::create(self.stderr_path(&id)).context("Couldn't open log file for STDERR")?;

        Command::new(&request.command)
            .args(&request.arguments)
            .stdout(stdout)
            .stderr(stderr)
            .spawn()
            .map(|mut child| {
                let pid = controlgroup::Pid::from(&child);

                cgroups
                    .add_task(pid)
                    .context("Couldn't add new process to the new Linux control group")
                    .map_err(|err| match &child.kill() {
                        Ok(_) => err,
                        Err(kerr) => anyhow!(
                            "Couldn't kill the process after failing to apply a control group: {}",
                            kerr
                        ),
                    })?;

                let process_id = id;
                let sys_pid = child.id();

                let mut map = self.processes.write().unwrap();
                (*map).insert(id, (child.id(), Running));

                let processes = Arc::clone(&self.processes);
                tokio::spawn(async move {
                    if let Ok(exit_status) = child.wait() {
                        let mut map = processes.write().unwrap();
                        (*map).insert(process_id, (sys_pid, Stopped(exit_status)));
                    } else {
                        warn!("Couldn't get the exit code for {}", process_id);
                    }
                });

                Ok(id)
            })
            .context("Couldn't spawn the process as specified")?
    }

    /// Stops a running process if it was started by this instance of the Runner
    ///
    /// # Panics
    ///
    /// Panics if called from outside of the Tokio runtime.
    pub async fn stop(&mut self, request: &StopRequest) -> Result<(), StopError> {
        if let Ok(id) = Uuid::parse_str(&request.id) {
            if let Some(pid) = self.pid_for_process(&id) {
                if let Some((_, Stopped(_))) = self.processes.read().unwrap().get(&id) {
                    return Err(TaskError {
                        description: "Process already stopped".to_string(),
                        variant: stop_error::Error::ProcessAlreadyStoppedError as i32,
                    }
                    .into());
                }

                let start = Instant::now();

                let sigkill = || -> Result<(), StopError> {
                    if signal::kill(Pid::from_raw(pid as i32), signal::Signal::SIGKILL).is_err() {
                        return Err(TaskError {
                            description: "Couldn't kill a process".to_string(),
                            variant: stop_error::Error::CouldntStopError as i32,
                        }
                        .into());
                    }

                    Ok(())
                };

                while let Some((_, Running)) = self.processes.read().unwrap().get(&id) {
                    if start.elapsed().as_secs() > 5 {
                        sigkill()?;
                        break;
                    } else {
                        match signal::kill(Pid::from_raw(pid as i32), signal::Signal::SIGTERM) {
                            Ok(_) => {
                                // let's give it a bit and re-check if the process
                                // is still there in the next run of this loop
                                tokio::time::sleep(Duration::from_millis(100)).await;
                            }
                            Err(err) => {
                                if let nix::Error::Sys(errno) = err {
                                    match errno {
                                        Errno::EACCES | Errno::ECHILD | Errno::EPERM => {
                                            return Err(anyhow!(errno.desc()).into());
                                        }
                                        Errno::ESRCH => break,
                                        _ => {
                                            sigkill()?;
                                            break;
                                        }
                                    }
                                }
                                break;
                            }
                        }
                    }
                }

                Ok(())
            } else {
                return Err(TaskError {
                    description: "Process not found".to_string(),
                    variant: stop_error::Error::ProcessNotFoundError as i32,
                }
                .into());
            }
        } else {
            return Err(TaskError {
                description: "Invalid process id".to_string(),
                variant: stop_error::Error::InvalidId as i32,
            }
            .into());
        }
    }

    /// Fetches the status of the process if it was started by this instanmce of the Runner.
    /// If the process has finished, returns an exit code or the signal that killed it
    pub async fn status(&mut self, request: &StatusRequest) -> Result<StatusResult, StatusError> {
        if let Ok(id) = Uuid::parse_str(&request.id) {
            let map = self.processes.read().unwrap();

            if let Some((_, process_status)) = map.get(&id) {
                match process_status {
                    Stopped(status) => {
                        let result = match status.code() {
                            Some(code) => {
                                status_result::Finish::Result(status_result::ExitResult {
                                    exit: Some(status_result::exit_result::Exit::Code(code)),
                                    kill: None,
                                })
                            }
                            None => match status.signal() {
                                Some(signal) => {
                                    status_result::Finish::Result(status_result::ExitResult {
                                        exit: None,
                                        kill: Some(status_result::exit_result::Kill::Signal(
                                            signal,
                                        )),
                                    })
                                }
                                None => {
                                    return Err(anyhow!(
                                        "Couldn't get exit code or the kill signal"
                                    )
                                    .into())
                                }
                            },
                        };

                        Ok(StatusResult {
                            finish: Some(result),
                        })
                    }
                    Running => Ok(StatusResult { finish: None }),
                }
            } else {
                return Err(TaskError {
                    description: "Process not found".to_string(),
                    variant: status_error::Error::ProcessNotFoundError as i32,
                }
                .into());
            }
        } else {
            return Err(TaskError {
                description: "Invalid process id".to_string(),
                variant: status_error::Error::InvalidId as i32,
            }
            .into());
        }
    }

    /// Returns a stream of stdout or stderr logs for a process. The stream implements
    /// futures::streams::Stream.
    pub async fn log(&mut self, request: &LogRequest) -> Result<LogStream, LogError> {
        let map = self.processes.read().unwrap();
        if let Ok(id) = Uuid::parse_str(&request.id) {
            if map.get(&id).is_some() {
                let maybe_descriptor = log_request::Descriptor::from_i32(request.descriptor);

                match maybe_descriptor {
                Some(descriptor) => {
                    let log_path = match descriptor {
                        log_request::Descriptor::Stdout => self.stdout_path(&id),
                        log_request::Descriptor::Stderr => self.stderr_path(&id),
                    };

                    Ok(LogStream::open(
                        id,
                        Arc::clone(&self.processes),
                        log_path.as_path(),
                        self.buffer_size.unwrap_or(256),
                    )?)
                }
                None => return Err(InternalError {
                    description: "Given descriptor is invalid. Are you using compatible version of the client?".to_string(),
                }.into())
            }
            } else {
                return Err(TaskError {
                    description: "Process not found".to_string(),
                    variant: log_error::Error::ProcessNotFoundError as i32,
                }
                .into());
            }
        } else {
            return Err(TaskError {
                description: "Invalid process id".to_string(),
                variant: log_error::Error::InvalidId as i32,
            }
            .into());
        }
    }

    /// Returns the path to stdout file for a process
    fn stdout_path(&self, id: &Uuid) -> PathBuf {
        let mut path = PathBuf::new();

        path.push(&self.log_dir);
        path.push(format!("{}.stdout.txt", id));

        path
    }

    /// Returns the path to stderr file for a process
    fn stderr_path(&self, id: &Uuid) -> PathBuf {
        let mut path = PathBuf::new();

        path.push(&self.log_dir);
        path.push(format!("{}.stderr.txt", id));

        path
    }

    /// Validates the "run process" request
    fn validate_run(&self, request: &RunRequest) -> Result<(), RunError> {
        if request.command.trim().is_empty() {
            return Err(TaskError {
                description: "Command name empty".to_string(),
                variant: run_error::Error::NameEmptyError as i32,
            }
            .into());
        }

        if let Some(Disk::MaxDisk(max)) = request.disk {
            if max > 1000 {
                return Err(TaskError {
                    description: "Max disk weight given greater than 1000 which is invalid"
                        .to_string(),
                    variant: run_error::Error::InvalidMaxDisk as i32,
                }
                .into());
            }
        }

        // Not validating arguments here since they really *can* be
        // anything. It's possible e.g. for some of them to be empty.

        Ok(())
    }

    /// Returns the PID for a given UUID id of the process
    fn pid_for_process(&self, id: &Uuid) -> Option<u32> {
        let processes = self.processes.read().unwrap();

        if let Some((pid, _)) = (*processes).get(id) {
            Some(*pid)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate sysinfo;
    extern crate tokio;

    use super::*;
    use futures::StreamExt;
    use service;
    use sysinfo::SystemExt;

    #[tokio::test]
    async fn proper_run_returns_correct_uuid() {
        let mut runner = Runner {
            log_dir: "tmp".to_string(),
            ..Default::default()
        };

        let request = RunRequest {
            command: "date".to_string(),
            ..Default::default()
        };

        runner.run(&request).await.unwrap();
    }

    #[tokio::test]
    async fn incorrect_run_returns_error() {
        let mut runner = Runner {
            log_dir: "tmp".to_string(),
            ..Default::default()
        };

        let request = RunRequest {
            command: "date".to_string(),
            disk: Some(service::run_request::Disk::MaxDisk(2000)),
            ..Default::default()
        };

        let res = runner.run(&request).await;

        assert!(res.is_err());
        assert!(
            res.err().unwrap().errors.unwrap()
                == service::run_response::run_error::Errors::RunError(
                    service::run_response::run_error::Error::InvalidMaxDisk as i32
                )
        );
    }

    #[tokio::test]
    async fn status_after_proper_long_run_works() {
        let mut runner = Runner {
            log_dir: "tmp".to_string(),
            ..Default::default()
        };

        let run_request = RunRequest {
            command: "sleep".to_string(),
            arguments: vec!["60".to_string()],
            ..Default::default()
        };

        let id = runner.run(&run_request).await.unwrap();

        let status_request = StatusRequest { id: id.to_string() };

        let response = runner.status(&status_request).await.unwrap();

        assert!(response.finish.is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn basic_stop_works() {
        let mut runner = Runner {
            log_dir: "tmp".to_string(),
            ..Default::default()
        };

        let run_request = RunRequest {
            command: "/usr/bin/env".to_string(),
            arguments: vec![
                "bash".to_string(),
                "-c".to_string(),
                "for i in $(seq 1 10000000000000); do echo $i; done".to_string(),
            ],
            ..Default::default()
        };

        let id = runner.run(&run_request).await.unwrap();
        let pid = runner.pid_for_process(&id).unwrap();

        let mut system = sysinfo::System::new();
        assert!(system.get_process(pid as i32).is_some());

        let stop_request = StopRequest { id: id.to_string() };
        let resp = runner.stop(&stop_request).await;

        resp.unwrap();

        system.refresh_processes();

        if let Some(process) = system.get_process(pid as i32) {
            assert!(
                process.status.as_ref().unwrap().to_string()
                    == sysinfo::ProcessStatus::Dead.to_string()
            );
        }
    }

    #[tokio::test]
    async fn gathering_logs_via_log_request_and_responce_stream_works_asynchronously() {
        let mut runner = Runner {
            log_dir: "tmp".to_string(),
            ..Default::default()
        };

        let run_request = RunRequest {
            command: "/usr/bin/env".to_string(),
            arguments: vec![
                "bash".to_string(),
                "-c".to_string(),
                "for i in $(seq 1 4); do echo $i; done".to_string(),
            ],
            ..Default::default()
        };

        let id = runner.run(&run_request).await.unwrap();

        // there's no need to wait for logs here since the
        // following log request is an async stream of values anyway
        // and it does its own waiting
        // (previous version had a thread::sleep here which was a remnant
        // of dev+debug time - it wasn't intended to stay here)

        let log_request = LogRequest {
            id: id.to_string(),
            descriptor: log_request::Descriptor::Stdout as i32,
        };

        let mut stream = runner.log(&log_request).await.unwrap();
        let first_value = stream.next().await;

        assert!(first_value.unwrap().unwrap() == "1\n2\n3\n4\n".as_bytes());
    }
}
