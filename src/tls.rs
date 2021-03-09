use crate::cipher::Cipher;
use anyhow::{bail, Context, Result};
use tokio_rustls::rustls::{
    internal::pemfile, Certificate as TlsCertificate, ClientConfig, PrivateKey, ServerConfig,
};

// Allow dead code for these functions as always some binary
// is not going to use one of them (e.g. client not using server config)

#[allow(dead_code)]
pub async fn client_config(
    cert: String,
    key: String,
    server_ca: String,
    cipher: Cipher,
) -> Result<ClientConfig> {
    let cert = tokio::fs::read(cert)
        .await
        .context("Couldn't read client certificate from file")?;
    let key = tokio::fs::read(key)
        .await
        .context("Couldn't read client private key from file")?;
    let server_ca_cert = tokio::fs::read(server_ca)
        .await
        .context("Couldn't read server CA from file")?;

    let (cert, key) =
        load_identity(cert, key).context("Couldn't load certificate and private key pair")?;
    let mut ca_cursor = std::io::Cursor::new(&server_ca_cert);

    let mut config = ClientConfig::with_ciphersuites(&[cipher.as_rustls_ciphersuite()]);

    config.set_protocols(&[Vec::from("h2")]);
    config
        .set_single_client_cert(cert, key)
        .context("Couldn't set client certificate")?;
    config.root_store.add_pem_file(&mut ca_cursor).unwrap();

    Ok(config)
}

#[allow(dead_code)]
pub async fn server_config(
    cert: String,
    key: String,
    client_ca: String,
    cipher: Cipher,
) -> Result<ServerConfig> {
    let cert = tokio::fs::read(cert)
        .await
        .context("Couldn't read server certificate from file")?;
    let key = tokio::fs::read(key)
        .await
        .context("Couldn't read server private key from file")?;
    let client_ca_cert = tokio::fs::read(client_ca)
        .await
        .context("Couldn't read client CA from file")?;

    let (cert, key) =
        load_identity(cert, key).context("Couldn't load certificate and private key pair")?;

    let mut client_ca_cert_cursor = std::io::Cursor::new(&client_ca_cert);

    let mut client_root_cert_store = tokio_rustls::rustls::RootCertStore::empty();
    if client_root_cert_store
        .add_pem_file(&mut client_ca_cert_cursor)
        .is_err()
    {
        bail!("Couldn't parse certificate");
    }

    let client_auth =
        tokio_rustls::rustls::AllowAnyAuthenticatedClient::new(client_root_cert_store);

    let mut config =
        ServerConfig::with_ciphersuites(client_auth, &[cipher.as_rustls_ciphersuite()]);

    config
        .set_single_cert(cert, key)
        .context("Couldn't set server certificate")?;
    config.set_protocols(&[Vec::from("h2")]);

    Ok(config)
}

fn load_identity(cert: Vec<u8>, key: Vec<u8>) -> Result<(Vec<TlsCertificate>, PrivateKey)> {
    let cert = {
        let mut cert = std::io::Cursor::new(&cert[..]);
        match pemfile::certs(&mut cert) {
            Ok(certs) => certs,
            Err(_) => bail!("Couldn't parse certificate"),
        }
    };

    let key = {
        let key = std::io::Cursor::new(&key[..]);
        match load_rustls_private_key(key) {
            Ok(key) => key,
            Err(e) => {
                bail!("Couldn't load private key: {}", e)
            }
        }
    };

    Ok((cert, key))
}

fn load_rustls_private_key(mut cursor: std::io::Cursor<&[u8]>) -> Result<PrivateKey> {
    if let Ok(mut keys) = pemfile::pkcs8_private_keys(&mut cursor) {
        if !keys.is_empty() {
            return Ok(keys.remove(0));
        }
    }

    bail!("Couldn't parse private key")
}
