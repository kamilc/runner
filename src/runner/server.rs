use crate::runner::service::{
    log_response, run_response, runner_server, status_response, LogRequest, LogResponse,
    RunRequest, RunResponse, StatusRequest, StatusResponse, StopRequest, StopResponse,
};
use crate::runner::Runner;
use futures::stream::Stream;
use futures::StreamExt;
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

        match self.runner.run(&run_request).await {
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

        match self.runner.stop(&stop_request).await {
            Ok(_) => Ok(Response::new(StopResponse { error: None })),
            Err(err) => Ok(Response::new(StopResponse { error: Some(err) })),
        }
    }

    async fn status(
        &self,
        request: Request<StatusRequest>,
    ) -> Result<Response<StatusResponse>, Status> {
        let status_request = request.into_inner();

        match self.runner.status(&status_request).await {
            Ok(result) => Ok(Response::new(StatusResponse {
                results: Some(status_response::Results::Result(result)),
            })),
            Err(err) => Ok(Response::new(StatusResponse {
                results: Some(status_response::Results::Error(err)),
            })),
        }
    }

    async fn log(
        &self,
        request: Request<LogRequest>,
    ) -> Result<Response<LogResponseStream>, Status> {
        let log_request = request.into_inner();

        match self.runner.log(&log_request).await {
            Ok(result) => {
                let ret = result.map(|item| match item {
                    Ok(data) => Ok(LogResponse {
                        results: Some(log_response::Results::Data(data)),
                    }),
                    Err(err) => Ok(LogResponse {
                        results: Some(log_response::Results::Error(err)),
                    }),
                });
                Ok(Response::new(Box::pin(ret)))
            }
            Err(err) => {
                let ret = futures::stream::unfold(Some(err), |state| async move {
                    if let Some(err) = state {
                        let resp = Ok(LogResponse {
                            results: Some(log_response::Results::Error(err)),
                        });
                        Some((resp, None))
                    } else {
                        None
                    }
                });

                Ok(Response::new(Box::pin(ret)))
            }
        }
    }
}
