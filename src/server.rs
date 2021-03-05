mod cipher;
mod cli;
mod runner;
mod tls;

use crate::runner::service::runner_server;
use anyhow::{Context, Result};
use cipher::Cipher;
use cli::server::Cli;
use runner::server::RunnerServer;
use structopt::StructOpt;
use tls::server_config;
use tonic::transport::{Server, ServerTlsConfig};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::from_args();

    start_server(args)
        .await
        .with_context(|| "Failed to run the runner server")?;

    Ok(())
}

async fn start_server(args: Cli) -> Result<()> {
    let addr = args
        .address
        .parse()
        .context("Failed to parse the server bind address")?;

    let server = RunnerServer::default();
    let tls_config = server_config(args.cert, args.key, args.client_ca, Cipher::ChaCha20).await?;
    let mut tls = ServerTlsConfig::new();

    tls.rustls_server_config(tls_config);

    println!("Starting Runner server at {}", &addr);

    Server::builder()
        .tls_config(tls)
        .with_context(|| "Failed to configure TLS")?
        .add_service(runner_server::RunnerServer::new(server))
        .serve(addr)
        .await?;

    Ok(())
}
