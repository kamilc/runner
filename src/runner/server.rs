use crate::runner::service::{
    log_response, run_response, runner_server, status_response, LogRequest, LogResponse,
    RunRequest, RunResponse, StatusRequest, StatusResponse, StopRequest, StopResponse,
};
use crate::runner::Runner;
use anyhow::Result;
use futures::stream::Stream;
use futures::StreamExt;
use prost::Message;
use std::pin::Pin;
use tonic::{Request, Response, Status};
use x509_parser::parse_x509_certificate;

type LogResponseStream = Pin<Box<dyn Stream<Item = Result<LogResponse, Status>> + Send + Sync>>;

#[derive(Default)]
pub struct RunnerServer {
    runner: Runner,
}

impl RunnerServer {
    fn authorize<T>(&self, request: &Request<T>) -> Result<(), Status>
    where
        T: Message,
    {
        request
            .peer_certs()
            .map_or(Err(Status::permission_denied("Unauthorized!")), |certs| {
                if certs.len() > 0 {
                    // let's authorize based on an immediate certificate in the chain:
                    let cert = &certs[0];

                    if let Ok((_, certificate)) = parse_x509_certificate(cert.get_ref()) {
                        if let Some(attr) = certificate.subject().iter_common_name().next() {
                            match attr.as_str() {
                                Ok(common_name) => {
                                    // A more real solution would keep the list of common names configured
                                    // somewhere and loaded into &self here and compare. For simplicity given
                                    // it's a proof-of-concept let's just hardcode the expected value:

                                    if common_name == "client" {
                                        Ok(())
                                    } else {
                                        Err(Status::permission_denied("Unauthorized!"))
                                    }
                                }
                                Err(_) => Err(Status::permission_denied("Unauthorized!")),
                            }
                        } else {
                            Err(Status::permission_denied("Unauthorized!"))
                        }
                    } else {
                        // this in theory shouldn't happen but let's
                        // return unauthorized here:
                        Err(Status::permission_denied(
                            "Unauthorized - couldn't parse certificate",
                        ))
                    }
                } else {
                    // this in theory shouldn't happen but let's
                    // return unauthorized here:
                    Err(Status::permission_denied(
                        "Unauthorized - no certificates found",
                    ))
                }
            })
    }
}

#[tonic::async_trait]
impl runner_server::Runner for RunnerServer {
    type LogStream = LogResponseStream;

    async fn run(&self, request: Request<RunRequest>) -> Result<Response<RunResponse>, Status> {
        self.authorize(&request)?;

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
        self.authorize(&request)?;

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
        self.authorize(&request)?;

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
        self.authorize(&request)?;

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
