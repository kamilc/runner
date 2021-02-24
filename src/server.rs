mod cli;
mod runner;

use anyhow::Result;
use structopt::StructOpt;

fn main() -> Result<()> {
    let _ = cli::server::Cli::from_args();

    Ok(())
}
