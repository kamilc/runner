use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Cli {
    /// Path to the client's CA root certificate
    #[structopt(long = "client-ca", env = "CLIENT_CA")]
    pub client_ca: String,

    /// Path to the server certificate
    #[structopt(long = "cert", env = "SERVER_CERT")]
    pub cert: String,

    /// Path to the server key
    #[structopt(long = "key", env = "SERVER_KEY")]
    pub key: String,

    /// gRPC address
    #[structopt(
        long = "address",
        env = "SERVER_ADDRESS",
        default_value = "[::1]:50051"
    )]
    pub address: String,
}
