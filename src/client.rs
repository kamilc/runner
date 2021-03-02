mod cli;
mod runner;

use cli::client::Cli;
use structopt::StructOpt;

fn main() {
    let _args = Cli::from_args();
}
