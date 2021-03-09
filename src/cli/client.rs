use crate::cipher::Cipher;
use clap::arg_enum;
use structopt::StructOpt;
use uuid::Uuid;

arg_enum! {
    #[derive(StructOpt, Debug)]
    pub enum Descriptor {
        Stdout,
        Stderr,
    }
}

#[derive(StructOpt, Debug)]
pub enum Command {
    /// Run a command
    Run {
        #[structopt(long)]
        /// Max memory share
        memory: Option<u64>,

        #[structopt(long)]
        /// Max cpu share
        cpu: Option<u64>,

        #[structopt(long)]
        /// Max read and write bytes/s for all disk devices
        disk: Option<u64>,

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
    pub server_ca: String,

    /// Path to the client certificate
    #[structopt(long = "cert", env = "CLIENT_CERT")]
    pub cert: String,

    /// Path to the client key
    #[structopt(long = "key", env = "CLIENT_KEY")]
    pub key: String,

    /// gRPC address
    #[structopt(
        long = "address",
        env = "SERVER_ADDRESS",
        default_value = "dns://[::1]:50051"
    )]
    pub address: String,

    /// Ciphersuite variant: chacha20 or aes
    #[structopt(long = "cipher", default_value = "chacha20", env = "CIPHER")]
    pub cipher: Cipher,

    #[structopt(subcommand)]
    pub command: Command,
}
