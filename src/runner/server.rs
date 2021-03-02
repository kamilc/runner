use crate::runner::service::runner_server;
use crate::runner::service::{
    LogRequest, LogResponse, RunRequest, RunResponse, StatusRequest, StatusResponse, StopRequest,
    StopResponse,
};
use futures::stream::Stream;
use std::pin::Pin;
use tonic::{Request, Response, Status};

#[derive(Default)]
pub struct RunnerServer;

type LogResponseStream = Pin<Box<dyn Stream<Item = Result<LogResponse, Status>> + Send + Sync>>;

#[tonic::async_trait]
impl runner_server::Runner for RunnerServer {
    type LogStream = LogResponseStream;

    async fn run(&self, _request: Request<RunRequest>) -> Result<Response<RunResponse>, Status> {
        unimplemented!();
    }

    async fn stop(&self, _request: Request<StopRequest>) -> Result<Response<StopResponse>, Status> {
        unimplemented!();
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
