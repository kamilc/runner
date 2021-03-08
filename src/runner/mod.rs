#[macro_use]
pub mod service;
pub mod server;

mod cgroups;
mod process_map;

use anyhow::{anyhow, Context, Result};
use cgroups::{apply_cgroup_pre_exec, create_cgroups};
use futures::stream::{unfold, Stream};
use log::{info, warn};
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
    run_response::{run_error, RunError},
    status_response::{status_error, status_result, StatusError, StatusResult},
    stop_response::{stop_error, StopError},
    InternalError, LogRequest, RunRequest, StatusRequest, StopRequest,
};
use std::fs::File;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::sync::RwLock;
use uuid::Uuid;

/// State of the log stream
#[derive(Clone)]
struct StreamState {
    processes: ProcessMap,
    file: Arc<RwLock<tokio::fs::File>>,
    buffer: Arc<RwLock<Vec<u8>>>,
    id: Uuid,
    close: bool,
}

/// Processes runner struct. Includes processes states and allows to
/// run them, stop, get their status and the stream of logs
#[derive(Clone, Debug)]
pub struct Runner {
    /// an internal map from UUID to ExitStatus
    processes: ProcessMap,

    /// where to keep process logs
    log_dir: String,

    /// The size of the buffer for streaming logs
    buffer_size: Option<usize>,
}

// A more real implementation would make sure that the log_dir exists
// Let's skip it for now as it's a proof-of-concept
impl Default for Runner {
    fn default() -> Self {
        Runner {
            processes: ProcessMap::default(),
            log_dir: "tmp".to_string(),
            buffer_size: Some(256),
        }
    }
}

impl Runner {
    /// Executes given command. Allows to specify the command name, arguments and
    /// resource constraints. Returns a UUID of the process or an error.
    ///
    /// # Panics
    ///
    /// Panics if called from outside of the Tokio runtime.
    pub async fn run(&self, request: &RunRequest) -> Result<Uuid, RunError> {
        if request.command.trim().is_empty() {
            return Err(run_error::Error::NameEmptyError.into());
        }

        let id = Uuid::new_v4();
        let mut cgroups = create_cgroups(request, &id).context("Couldn't create a cgroup")?;
        let stdout =
            File::create(self.stdout_path(&id)).context("Couldn't open log file for STDOUT")?;
        let stderr =
            File::create(self.stderr_path(&id)).context("Couldn't open log file for STDERR")?;

        let mut cmd = Command::new(&request.command);

        cmd.args(&request.arguments);
        cmd.stdout(stdout);
        cmd.stderr(stderr);

        if let Some(cgroup) = &cgroups.cpu() {
            apply_cgroup_pre_exec(&mut cmd, *cgroup);
        }

        if let Some(cgroup) = &cgroups.memory() {
            apply_cgroup_pre_exec(&mut cmd, *cgroup);
        }

        if let Some(cgroup) = &cgroups.blkio() {
            apply_cgroup_pre_exec(&mut cmd, *cgroup);
        }

        let spawn = cmd.spawn();

        match spawn {
            Ok(mut child) => {
                let sys_pid: u32 = child.id().unwrap();
                let processes = Arc::clone(&self.processes);

                let mut map = self.processes.write().await;
                (*map).insert(id, (child.id().unwrap(), Running));

                info!("Spawned child {} for {}", &sys_pid, &id);

                tokio::spawn(async move {
                    // A fuller solution would be to kill child processes upon us
                    // receiving SIGINT, SIGTERM or SIGQUIT. In order to do so
                    // properly, a PID namespace would need to be unshared - to
                    // defend against the "double-fork" daemoning where the second
                    // one in reparented to the init process. This stays out of
                    // scope of this work though.

                    match child.wait().await {
                        Ok(exit_status) => {
                            let mut map = processes.write().await;
                            (*map).insert(id, (sys_pid, Stopped(exit_status)));
                        }
                        Err(_) => {
                            warn!("Couldn't get the exit status for {}", &id);
                        }
                    }

                    if let Err(err) = cgroups.delete() {
                        warn!(
                            "Couldn't delete control group for {}: {}",
                            &id,
                            err.to_string()
                        )
                    }
                });

                Ok(id)
            }
            Err(err) => Err(err.into()),
        }
    }

    /// Stops a running process if it was started by this instance of the Runner
    ///
    /// # Panics
    ///
    /// Panics if called from outside of the Tokio runtime.
    pub async fn stop(&self, request: &StopRequest) -> Result<(), StopError> {
        if let Ok(id) = Uuid::parse_str(&request.id) {
            if let Some(pid) = self.pid_for_process(&id).await {
                if let Some((_, Stopped(_))) = self.processes.read().await.get(&id) {
                    return Err(stop_error::Error::ProcessAlreadyStoppedError.into());
                }

                let start = Instant::now();

                let sigkill = || -> Result<(), StopError> {
                    if signal::kill(Pid::from_raw(pid as i32), signal::Signal::SIGKILL).is_err() {
                        return Err(stop_error::Error::CouldntStopError.into());
                    }

                    Ok(())
                };

                while let Some((_, Running)) = self.processes.read().await.get(&id) {
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
                Err(stop_error::Error::ProcessNotFoundError.into())
            }
        } else {
            Err(stop_error::Error::InvalidId.into())
        }
    }

    /// Fetches the status of the process if it was started by this instanmce of the Runner.
    /// If the process has finished, returns an exit code or the signal that killed it
    pub async fn status(&self, request: &StatusRequest) -> Result<StatusResult, StatusError> {
        if let Ok(id) = Uuid::parse_str(&request.id) {
            let map = self.processes.read().await;

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
                Err(status_error::Error::ProcessNotFoundError.into())
            }
        } else {
            Err(status_error::Error::InvalidId.into())
        }
    }

    /// Returns a stream of stdout or stderr logs for a process. The stream implements
    /// futures::streams::Stream.
    ///
    /// # Panics
    ///
    /// Panics if called from outside of the Tokio runtime.
    pub async fn log(
        &self,
        request: &LogRequest,
    ) -> Result<
        std::pin::Pin<Box<dyn Stream<Item = Result<Vec<u8>, LogError>> + Send + Sync>>,
        LogError,
    > {
        let map = self.processes.read().await;

        if let Ok(id) = Uuid::parse_str(&request.id) {
            if map.get(&id).is_some() {
                let maybe_descriptor = log_request::Descriptor::from_i32(request.descriptor);

                match maybe_descriptor {
                    Some(descriptor) => {
                        let log_path = match descriptor {
                            log_request::Descriptor::Stdout => self.stdout_path(&id),
                            log_request::Descriptor::Stderr => self.stderr_path(&id),
                        };

                        let file = Arc::new(RwLock::new(
                            tokio::fs::File::open(&log_path).await.context("Couldn't open log file")?,
                        ));

                        let buffer_size = self.buffer_size.unwrap_or(256);

                        let buffer: Arc<RwLock<Vec<u8>>> = Arc::new(RwLock::new(Vec::with_capacity(buffer_size)));
                        buffer.write().await.resize_with(buffer_size, Default::default);

                        let state = StreamState {
                            processes: Arc::clone(&self.processes),
                            file,
                            buffer,
                            id,
                            close: false
                        };

                        Ok(Box::pin(unfold(state, |state| async move {
                            if state.close {
                                return None;
                            }

                            let mut log = state.file.write().await;
                            let mut buf = state.buffer.write().await;

                            loop {
                                if let Ok(bytes) = (*log).read(&mut *buf).await {
                                    if bytes > 0 {
                                        let data = (*buf)[0..bytes].to_vec();

                                        return Some((Ok(data), state.clone()));
                                    } else if let Some((_, Stopped(_))) = state.processes.read().await.get(&state.id) {
                                        return None;
                                    } else {
                                        tokio::time::sleep(Duration::from_millis(100)).await;
                                    }
                                } else {
                                    let state = StreamState { close: true, ..state.clone() };

                                    return Some((Err(anyhow!("Error reading from log file").into()), state ));
                                }
                            }

                        })))
                    }
                    None => Err(InternalError {
                        description: "Given descriptor is invalid. Are you using compatible version of the client?".to_string(),
                    }.into())
                }
            } else {
                Err(log_error::Error::ProcessNotFoundError.into())
            }
        } else {
            Err(log_error::Error::InvalidId.into())
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

    /// Returns the PID for a given UUID id of the process
    async fn pid_for_process(&self, id: &Uuid) -> Option<u32> {
        let processes = self.processes.read().await;

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
    use sysinfo::SystemExt;

    #[tokio::test]
    async fn proper_run_returns_correct_uuid() {
        let runner = Runner {
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
        let runner = Runner {
            log_dir: "tmp".to_string(),
            ..Default::default()
        };

        let request = RunRequest {
            command: "".to_string(),
            ..Default::default()
        };

        let res = runner.run(&request).await;

        assert!(res.is_err());
        assert!(
            res.err().unwrap().errors.unwrap()
                == service::run_response::run_error::Errors::RunError(
                    service::run_response::run_error::Error::NameEmptyError as i32
                )
        );
    }

    #[tokio::test]
    async fn status_after_proper_long_run_works() {
        let runner = Runner {
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
        let runner = Runner {
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
        let pid = runner.pid_for_process(&id).await.unwrap();

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
        let runner = Runner {
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

    #[tokio::test]
    async fn closed_processes_have_streams_with_an_end() {
        let runner = Runner {
            log_dir: "tmp".to_string(),
            ..Default::default()
        };

        // let's use /usr/bin/env here not to assume where echo resides
        // env is most often under /usr/bin
        let run_request = RunRequest {
            command: "/usr/bin/env".to_string(),
            arguments: vec![
                "bash".to_string(),
                "-c".to_string(),
                "echo test".to_string(),
            ],
            ..Default::default()
        };

        let id = runner.run(&run_request).await.unwrap();

        let log_request = LogRequest {
            id: id.to_string(),
            descriptor: log_request::Descriptor::Stdout as i32,
        };

        let mut stream = runner.log(&log_request).await.unwrap();
        let first_value = stream.next().await;

        assert!(first_value.unwrap().unwrap() == "test\n".as_bytes());
        assert!(stream.next().await.is_none());
    }
}
