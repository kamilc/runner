mod cipher;
mod cli;
mod runner;
mod tls;

use crate::runner::service::runner_server;
use anyhow::{Context, Result};
use cli::server::Cli;
use log::warn;
use nix::sys::signal::signal;
use nix::sys::signal::{SigHandler, Signal};
use runner::server::RunnerServer;
use signal_hook::{
    consts::signal::{SIGINT, SIGQUIT, SIGTERM},
    iterator::Signals,
};
use structopt::StructOpt;
use tls::server_config;
use tonic::transport::{Server, ServerTlsConfig};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::from_args();

    if !args.silent {
        pretty_env_logger::init();
    }

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
    let tls_config = server_config(args.cert, args.key, args.client_ca, args.cipher).await?;
    let mut tls = ServerTlsConfig::new();

    tls.rustls_server_config(tls_config);

    // As processes are not awaited in a blocking way but using try_wait in a loop
    // (to keep the threads in a pool available), let's make sure we're not gonna
    // produce zombie processes here.

    let mut signals = Signals::new(&[SIGINT, SIGTERM, SIGQUIT])?;
    tokio::spawn(async move {
        for sig in signals.forever() {
            println!("\nReceived signal {:?}. Cleaning up now.", sig);

            unsafe {
                if let Err(err) = signal(Signal::SIGCHLD, SigHandler::SigIgn) {
                    warn!("Couldn't set-up the children cleanups: {}", err.to_string());
                }
            }

            std::process::exit(0);
        }
    });

    println!("Starting Runner server at {}", &addr);

    Server::builder()
        .tls_config(tls)
        .with_context(|| "Failed to configure TLS")?
        .add_service(runner_server::RunnerServer::new(server))
        .serve(addr)
        .await?;

    Ok(())
}
