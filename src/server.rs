mod cli;
mod runner;

use anyhow::{Context, Result};
use cli::server::Cli;
use futures::stream::Stream;
use runner::service::runner_server;
use runner::service::{
    LogRequest, LogResponse, RunRequest, RunResponse, StatusRequest, StatusResponse, StopRequest,
    StopResponse,
};
use std::pin::Pin;
use structopt::StructOpt;
use tonic::transport::{Certificate, Identity, Server, ServerTlsConfig};
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

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::from_args();

    start_server(args)
        .await
        .with_context(|| "Failed to run the runner server")?;

    Ok(())
}

async fn start_server(args: Cli) -> Result<()> {
    let cert = tokio::fs::read(args.cert).await?;
    let key = tokio::fs::read(args.key).await?;

    let server_identity = Identity::from_pem(cert, key);

    let client_ca_cert = tokio::fs::read(args.client_ca).await?;
    let client_ca_cert = Certificate::from_pem(client_ca_cert);

    let addr = args
        .address
        .parse()
        .context("Failed to parse the server bind address")?;

    let server = RunnerServer::default();

    let tls = ServerTlsConfig::new()
        .identity(server_identity)
        .client_ca_root(client_ca_cert);

    println!("Starting Runner server at {}", &addr);

    Server::builder()
        .tls_config(tls)
        .with_context(|| "Failed to configure TLS")?
        .add_service(runner_server::RunnerServer::new(server))
        .serve(addr)
        .await?;

    Ok(())
}
