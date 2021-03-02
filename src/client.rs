mod cli;
mod runner;

use anyhow::{Context, Result};
use cli::client::{Cli, Command};
use structopt::StructOpt;
use tonic::transport::Uri;
use tonic::transport::{Certificate, Channel, ClientTlsConfig, Identity};
use uuid::Uuid;

use crate::runner::service::{
    log_request, log_response, run_request, run_response, runner_client, runner_server,
    status_response, LogRequest, LogResponse, RunRequest, RunResponse, StatusRequest,
    StatusResponse, StopRequest, StopResponse,
};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::from_args();

    let cert = tokio::fs::read(args.cert).await?;
    let key = tokio::fs::read(args.key).await?;

    let client_identity = Identity::from_pem(cert, key);

    let server_ca_cert = tokio::fs::read(args.server_ca).await?;
    let server_ca_cert = Certificate::from_pem(server_ca_cert);

    let tls = ClientTlsConfig::new()
        .domain_name("localhost")
        .ca_certificate(server_ca_cert)
        .identity(client_identity);

    let uri = args
        .address
        .parse::<Uri>()
        .context("Invalid address given")?;

    let channel = Channel::builder(uri).tls_config(tls)?.connect().await?;

    let mut client = runner_client::RunnerClient::new(channel);

    match args.command {
        Command::Run {
            memory,
            disk,
            cpu,
            command,
            args,
        } => {
            let request = tonic::Request::new(RunRequest {
                command,
                arguments: args,
                disk: disk.map(|v| run_request::Disk::MaxDisk(v)),
                memory: memory.map(|v| run_request::Memory::MaxMemory(v)),
                cpu: cpu.map(|v| run_request::Cpu::MaxCpu(v)),
            });

            let response = client.run(request).await?;

            match response.into_inner().results.unwrap() {
                run_response::Results::Id(id) => println!("{}", id),
                run_response::Results::Error(err) => println!("Error: {}", err.description),
            }
        }
        Command::Stop { id } => {
            let request = tonic::Request::new(StopRequest { id: id.to_string() });

            let response = client.stop(request).await?;

            match response.into_inner().error {
                Some(err) => println!("Error: {}", err.description),
                None => println!("Stopped"),
            }
        }
        Command::Log { id, descriptor } => println!("todo"),
        Command::Status { id } => println!("todo"),
    };

    Ok(())
}
