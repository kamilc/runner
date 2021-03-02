use clap::arg_enum;
use structopt::StructOpt;
use uuid::Uuid;

arg_enum! {
    #[derive(StructOpt, Debug)]
    enum Descriptor {
        Stdout,
        Stderr,
    }
}

#[derive(StructOpt, Debug)]
enum Command {
    /// Run a command
    Run {
        #[structopt(long)]
        /// Max memory share
        memory: Option<u32>,

        #[structopt(long)]
        /// Max cpu share
        cpu: Option<u32>,

        #[structopt(long)]
        /// Max disk weight
        disk: Option<u32>,

        /// Command to run
        command: String,

        /// Commands arguments
        args: Vec<String>,
    },
    /// Stop a command
    Stop {
        /// Task ID as returned from `run`
        id: Uuid,
    },
    /// Get command's status
    Status {
        /// Task ID as returned from `run`
        id: Uuid,
    },
    /// Follow command's logs
    Log {
        /// Task ID as returned from `run`
        id: Uuid,

        /// Process output descriptor (stdout | stderr)
        descriptor: Descriptor,
    },
}

#[derive(StructOpt, Debug)]
pub struct Cli {
    /// Path to the server's CA root certificate
    #[structopt(long = "server-ca", env = "SERVER_CA")]
    server_ca: String,

    /// Path to the client certificate
    #[structopt(long = "cert", env = "CLIENT_CERT")]
    cert: String,

    /// Path to the client key
    #[structopt(long = "key", env = "CLIENT_KEY")]
    key: String,

    /// gRPC address
    #[structopt(
        long = "address",
        env = "SERVER_ADDRESS",
        default_value = "[::1]:50051"
    )]
    address: String,

    #[structopt(subcommand)]
    command: Command,
}
