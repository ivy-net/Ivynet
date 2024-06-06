use tonic::transport::{Certificate, Channel, ClientTlsConfig, Endpoint};
use tracing::{debug, info};

pub use tonic::{transport::Uri, Request, Response};

pub fn create_channel(uri: &Uri, tls_ca: Option<&String>) -> Channel {
    debug!("Initializing GRPC channel: {}", uri);
    let endpoint = Endpoint::from_shared(uri.to_string()).expect("invalid backend URI");
    let endpoint = if let Some(ca) = tls_ca {
        let ca = std::fs::read_to_string(ca).expect("can't read CA certificate");
        let tls = ClientTlsConfig::new().ca_certificate(Certificate::from_pem(ca));
        endpoint.tls_config(tls).expect("invalid CA certificate")
    } else {
        endpoint
    }
    .timeout(std::time::Duration::from_secs(5));
    info!("Initialized GRPC channel: {}", uri);
    endpoint.connect_lazy()
}
