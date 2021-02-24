pub mod service {
    tonic::include_proto!("service");
}

use anyhow::Result;
use service::{
    log_response, run_response, status_response, stop_response, LogRequest, RunRequest,
    StatusRequest, StopRequest,
};

// todo: implement std::iter::Iterator for this stream
// with Item=LogResponse
pub struct LogStream;

pub struct Runner;

impl Runner {
    // todo: implement me
    pub fn run(&mut self, _request: &RunRequest) -> Result<String, run_response::Error> {
        Ok("".to_string())
    }

    // todo: implement me
    pub fn stop(&mut self, _request: &StopRequest) -> Result<(), stop_response::Error> {
        Ok(())
    }

    // todo: implement me
    pub fn status(
        &mut self,
        _request: &StatusRequest,
    ) -> Result<status_response::StatusResult, status_response::Error> {
        Ok(status_response::StatusResult {
            status: 0,
            exit_code: 0,
        })
    }

    pub fn log(&mut self, _request: &LogRequest) -> Result<LogStream, log_response::Error> {
        Ok(LogStream {})
    }
}
