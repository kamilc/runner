use anyhow::{Context, Result, bail};
use tokio_rustls::{
    rustls::{ClientConfig, NoClientAuth, ServerConfig, Session},
    webpki::DNSNameRef,
    TlsAcceptor as RustlsAcceptor, TlsConnector as RustlsConnector,
};
use tonic::transport::{
    server::{Connected},
    Certificate, Identity,
};
use tonic::transport::{Certificate};

pub async fn server_config(cert: String, key: String, client_ca: String) -> Result<ServerConfig> {
    let cert = tokio::fs::read(cert).await?;
    let key = tokio::fs::read(key).await?;

    let server_identity = Identity::from_pem(cert, key);

    let (cert, key) = load_identity(server_identity)?;

    let client_ca_cert = tokio::fs::read(client_ca).await?;
    let client_ca_cert = Certificate::from_pem(client_ca_cert);

    let mut client_root_cert_store = tokio_rustls::rustls::RootCertStore::empty();
    if client_root_cert_store.add_pem_file(&mut client_ca_cert).is_err() {
        bail!("Couldn't parse certificate");
    }

    let client_auth = tokio_rustls::rustls::AllowAnyAuthenticatedClient::new(client_root_cert_store);

    let config = ServerConfig::new(client_auth)
    config.set_single_cert(cert, key)?;
    config.set_protocols(&[Vec::from(&ALPN_H2[..])]);

    Ok(config)
}

fn load_identity(
    identity: Identity,
) -> Result<(Vec<Certificate>, PrivateKey), crate::Error> {
    let cert = {
        let mut cert = std::io::Cursor::new(&identity.cert.pem[..]);
        match pemfile::certs(&mut cert) {
            Ok(certs) => certs,
            Err(_) => return Err(Box::new(TlsError::CertificateParseError)),
        }
    };

    let key = {
        let key = std::io::Cursor::new(&identity.key[..]);
        match load_rustls_private_key(key) {
            Ok(key) => key,
            Err(e) => {
                return Err(e);
            }
        }
    };

    Ok((cert, key))
}
