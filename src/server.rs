mod cli;
mod runner;

use anyhow::{Context, Result};
use cli::server::Cli;
use std::time::Duration;
use structopt::StructOpt;

#[derive(Default)]
pub struct RunnerServer;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::from_args();

    start_server(args)
        .await
        .with_context(|| "Failed to run the runner server")?;

    Ok(())
}

async fn start_server(_args: Cli) -> Result<()> {
    // todo: implement me
    tokio::time::sleep(Duration::from_millis(9999999999)).await;

    Ok(())
}
