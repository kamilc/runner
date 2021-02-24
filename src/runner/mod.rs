pub mod service {
    tonic::include_proto!("service");
}

use anyhow::Result;
use service::{
    status_response::Status, LogRequest, RunRequest, RunResponse, StatusRequest, StatusResponse,
    StopRequest, StopResponse,
};

// todo: implement std::iter::Iterator for this stream
// with Item=LogResponse
pub struct LogStream;

pub struct Runner;

impl Runner {
    // todo: implement me
    pub fn run(&mut self, _request: &RunRequest) -> Result<RunResponse> {
        Ok(RunResponse {
            id: "".to_string(),
            error: None,
            general_error: None,
        })
    }

    // todo: implement me
    pub fn stop(&mut self, _request: &StopRequest) -> Result<StopResponse> {
        Ok(StopResponse {
            error: None,
            general_error: None,
        })
    }

    // todo: implement me
    pub fn status(&mut self, _request: &StatusRequest) -> Result<StatusResponse> {
        Ok(StatusResponse {
            error: None,
            general_error: None,
            exit_code: 0,
            status: Status::Running as i32,
        })
    }

    pub fn log(&mut self, _request: &LogRequest) -> Result<LogStream> {
        Ok(LogStream)
    }
}
