use crate::runner::service::{
    run_response, runner_server, LogRequest, LogResponse, RunRequest, RunResponse, StatusRequest,
    StatusResponse, StopRequest, StopResponse,
};
use crate::runner::Runner;
use futures::stream::Stream;
use std::pin::Pin;
use tonic::{Request, Response, Status};

#[derive(Default)]
pub struct RunnerServer {
    runner: Runner,
}

type LogResponseStream = Pin<Box<dyn Stream<Item = Result<LogResponse, Status>> + Send + Sync>>;

#[tonic::async_trait]
impl runner_server::Runner for RunnerServer {
    type LogStream = LogResponseStream;

    async fn run(&self, request: Request<RunRequest>) -> Result<Response<RunResponse>, Status> {
        let run_request = request.into_inner();

        match self.runner.run(&run_request) {
            Ok(id) => Ok(Response::new(RunResponse {
                results: Some(run_response::Results::Id(id.to_string())),
            })),
            Err(err) => Ok(Response::new(RunResponse {
                results: Some(run_response::Results::Error(err)),
            })),
        }
    }

    async fn stop(&self, request: Request<StopRequest>) -> Result<Response<StopResponse>, Status> {
        let stop_request = request.into_inner();

        match self.runner.stop(&stop_request) {
            Ok(_) => Ok(Response::new(StopResponse { error: None })),
            Err(err) => Ok(Response::new(StopResponse { error: Some(err) })),
        }
    }

    async fn status(
        &self,
        _request: Request<StatusRequest>,
    ) -> Result<Response<StatusResponse>, Status> {
        unimplemented!();
    }

    async fn log(
        &self,
        _request: Request<LogRequest>,
    ) -> Result<Response<LogResponseStream>, Status> {
        unimplemented!();
    }
}
