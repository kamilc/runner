mod cgroups;

#[macro_use]
mod service;

use anyhow::{anyhow, Context, Result};
use cgroups::create_cgroups;
use controlgroup;
use futures::stream::Stream;
use futures::task::Poll;
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
use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::default::Default;
use std::fs::File;
use std::io::Read;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::pin::Pin;
use std::process::ExitStatus;
use std::process::{Child, Command};
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::sync::RwLock;
use std::thread;
use std::time::Duration;
use sysinfo::{ProcessExt, SystemExt};
use uuid::Uuid;

type ProcessMap = Arc<RwLock<HashMap<String, (u32, Option<ExitStatus>)>>>;

struct LogStream {
    map: ProcessMap,
    file: Arc<RwLock<File>>,
    process_id: String,
    closed: bool,
}

impl LogStream {
    pub fn open(process_id: String, processes: ProcessMap, path: PathBuf) -> Result<Self> {
        let file = File::open(&path)?;

        Ok(LogStream {
            map: processes,
            file: Arc::new(RwLock::new(file)),
            process_id: process_id,
            closed: false,
        })
    }
}

impl Stream for LogStream {
    type Item = Result<String, LogError>;

    fn poll_next(
        self: Pin<&mut Self>,
        _cx: &mut futures::task::Context,
    ) -> Poll<Option<Self::Item>> {
        if self.closed {
            return Poll::Ready(None);
        }

        let this = Pin::<&mut LogStream>::into_inner(self);

        if let Some((_, Some(_))) = this.map.read().unwrap().get(&this.process_id) {
            // exit code is present, process has ended, we're finishing here
            if this.closed {
                return Poll::Ready(None);
            }
        }

        let mut buffer = [0; 32];

        let mut file_lock = this.file.write().unwrap();
        let file = file_lock.borrow_mut();

        if let Ok(bytes) = file.read(&mut buffer) {
            if bytes > 0 {
                match std::str::from_utf8(&buffer[0..bytes]) {
                    Ok(s) => Poll::Ready(Some(Ok(s.to_string()))),
                    Err(err) => {
                        // we can't decode the string. let's keep handling of
                        // non-utf8 compatible coding as out-of-scope here
                        this.closed = true;
                        Poll::Ready(Some(Err(err.into())))
                    }
                }
            } else {
                // looks like there's no new data for now
                Poll::Pending
            }
        } else {
            this.closed = true;

            Poll::Ready(Some(Err(anyhow!("Error reading from log file").into())))
        }
    }
}

#[derive(Default)]
pub struct Runner {
    /// an internal map from UUID to ExitStatus
    processes: ProcessMap,

    /// where to keep process logs
    log_dir: String,
}

impl Runner {
    pub fn run(&mut self, request: &RunRequest) -> Result<String, RunError> {
        self.validate_run(&request)?;

        let id = Uuid::new_v4().to_string();
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

                insert_process(processes.clone(), &process_id, sys_pid);

                thread::Builder::new()
                    .spawn(move || {
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
                let mut system = sysinfo::System::new();
                system.refresh_processes();

                if let Some(process) = system.get_process(pid as i32) {
                    process.kill(sysinfo::Signal::Term);
                    thread::sleep(Duration::from_millis(200));
                } else {
                    send.send(()).unwrap();
                }
            });

            if let Err(_) = recv.recv_timeout(Duration::from_millis(5000)) {
                let mut system = sysinfo::System::new();

                system.refresh_processes();

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

    pub fn log(&mut self, request: &LogRequest) -> Result<LogStream, LogError> {
        let map = self.processes.read().unwrap();

        if let Some(_) = map.get(&request.id) {
            let maybe_descriptor = log_request::Descriptor::from_i32(request.descriptor);

            match maybe_descriptor {
                Some(descriptor) => {
                    let log_path = match descriptor {
                        log_request::Descriptor::Stdout => self.stdout_path(&request.id),
                        log_request::Descriptor::Stderr => self.stderr_path(&request.id),
                    };

                    Ok(LogStream::open(
                        request.id.clone(),
                        self.processes.clone(),
                        log_path,
                    )?)
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

    pub fn pid_for_process(&self, id: &String) -> Option<u32> {
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

#[cfg(test)]
mod tests {
    extern crate tokio;

    use super::*;
    use futures::StreamExt;
    use service;
    use sysinfo;
    use uuid::Uuid;

    #[test]
    fn proper_run_returns_correct_uuid() {
        let mut runner = Runner {
            log_dir: "tmp".to_string(),
            ..Default::default()
        };

        let request = RunRequest {
            command: "date".to_string(),
            ..Default::default()
        };

        let mut id = runner.run(&request).unwrap();

        assert!(Uuid::parse_str(&id).is_ok());
    }

    #[test]
    fn status_after_proper_long_run_works() {
        let mut runner = Runner {
            log_dir: "tmp".to_string(),
            ..Default::default()
        };

        let run_request = RunRequest {
            command: "sleep".to_string(),
            arguments: vec!["60".to_string()],
            ..Default::default()
        };

        let mut id = runner.run(&run_request).unwrap();

        let status_request = StatusRequest { id: id };

        let response = runner.status(&status_request).unwrap();

        assert!(response.status == service::status_response::Status::Running as i32);
    }

    #[test]
    fn basic_stop_works() {
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

        let mut id = runner.run(&run_request).unwrap();
        let pid = runner.pid_for_process(&id).unwrap();

        let mut system = sysinfo::System::new();
        assert!(system.get_process(pid as i32).is_some());

        let stop_request = StopRequest { id: id };
        let resp = runner.stop(&stop_request);

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
    async fn simple_case_of_logs_works() {
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

        let mut id = runner.run(&run_request).unwrap();

        std::thread::sleep(Duration::from_millis(100));

        let log_request = LogRequest {
            id: id,
            descriptor: log_request::Descriptor::Stdout as i32,
        };

        let mut stream = runner.log(&log_request).unwrap();
        let first_value = stream.next().await;

        assert!(first_value.unwrap().unwrap() == "1\n2\n3\n4\n");
    }
}
