use hyper_util::rt::TokioIo;
use tokio::net::UnixStream;
use tonic::transport::{Certificate, Channel, ClientTlsConfig, Endpoint};
use tower::service_fn;
use tracing::{debug, info};

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

pub async fn create_channel(
    source: Source,
    tls_ca: Option<&String>,
) -> Result<Channel, ClientError> {
    debug!("Initializing GRPC channel: {:?}", source);
    let endpoint = match source {
        Source::Uri(ref uri) => {
            Endpoint::from_shared(uri.to_string()).expect("invalid backend URI")
        }
        Source::Path(ref path) =>
        {
            #[allow(clippy::expect_fun_call)]
            Endpoint::try_from("http://[::]:50050")
                .expect(&format!("unable to open socket at {path}"))
        }
    };
    let endpoint = if let Some(ca) = tls_ca {
        let ca = std::fs::read_to_string(ca).expect("can't read CA certificate");
        let tls = ClientTlsConfig::new().ca_certificate(Certificate::from_pem(ca));
        endpoint.tls_config(tls).expect("invalid CA certificate")
    } else {
        endpoint
    }
    .timeout(std::time::Duration::from_secs(60));
    info!("Initialized GRPC channel: {:?}", source);
    match source {
        Source::Path(ref path) => {
            let mut client = Some(TokioIo::new(UnixStream::connect(&path).await?));
            Ok(endpoint.connect_with_connector_lazy(service_fn(move |_: Uri| {
                // Connect to a Uds socket
                let client = client.take();

                async move {
                    if let Some(client) = client {
                        Ok(TokioIo::new(client))
                    } else {
                        Err(std::io::Error::new(std::io::ErrorKind::Other, "Client already taken"))
                    }
                }
            })))
        }
        _ => Ok(endpoint.connect_lazy()),
    }
}
