use tonic::transport::{Certificate, Channel, ClientTlsConfig, Endpoint};
use tracing::debug;

pub use tonic::{transport::Uri, Request, Response};

#[derive(Debug)]
pub enum Source {
    Uri(Uri),
    Path(String),
}

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error(transparent)]
    SocketError(#[from] std::io::Error),
}

pub async fn create_channel(source: Uri, tls_ca: Option<String>) -> Result<Channel, ClientError> {
    debug!("Initializing GRPC channel: {:?}", source);
    let endpoint = Endpoint::from_shared(source.to_string()).expect("invalid backend URI");
    let endpoint = if let Some(ca) = tls_ca {
        let ca = std::fs::read_to_string(ca).expect("can't read CA certificate");
        let tls = ClientTlsConfig::new().ca_certificate(Certificate::from_pem(ca));
        endpoint.tls_config(tls).expect("invalid CA certificate")
    } else {
        endpoint
    }
    .timeout(std::time::Duration::from_secs(5 * 60));
    debug!("Initialized GRPC channel: {:?}", source);
    Ok(endpoint.connect_lazy())
}
