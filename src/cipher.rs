use clap::arg_enum;
use rustls::ciphersuite::{TLS13_AES_256_GCM_SHA384, TLS13_CHACHA20_POLY1305_SHA256};
use rustls::SupportedCipherSuite;
use structopt::StructOpt;

arg_enum! {
    #[derive(StructOpt, Debug)]
        pub enum Cipher {
          ChaCha20,
          Aes
        }
}

impl Cipher {
    pub fn as_rustls_ciphersuite(&self) -> &'static SupportedCipherSuite {
        match self {
            Cipher::ChaCha20 => &TLS13_CHACHA20_POLY1305_SHA256,
            Cipher::Aes => &TLS13_AES_256_GCM_SHA384,
        }
    }
}
