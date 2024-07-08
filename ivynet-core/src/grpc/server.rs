use core::fmt;
use std::{
    convert::Infallible,
    fmt::{Display, Formatter},
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
    time::Duration,
};

use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::{
    body::BoxBody,
    codegen::{
        http::{Request, Response},
        Service,
    },
    server::NamedService,
    transport::{server::Router, Body, Identity, Server as TonicServer, ServerTlsConfig},
};
use tracing::info;

#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error(transparent)]
    TonicTransportError(#[from] tonic::transport::Error),

    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

struct Socket(PathBuf);

pub struct Server {
    pub router: Router,
    socket: Option<Socket>,
}

impl Drop for Socket {
    fn drop(&mut self) {
        if let Err(e) = std::fs::remove_file(&self.0) {
            eprintln!("Failed to remove socket file: {}", e);
        }
    }
}

pub enum Endpoint {
    Port(u16),
    Path(String),
}

impl Display for Endpoint {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Endpoint::Port(port) => write!(f, "port {}", port),
            Endpoint::Path(path) => write!(f, "path {}", path),
        }
    }
}

impl Server {
    pub fn new<S>(service: S, cert_path: Option<String>, key_path: Option<String>) -> Self
    where
        S: Service<Request<Body>, Response = Response<BoxBody>, Error = Infallible>
            + NamedService
            + Clone
            + Send
            + 'static,
        S::Future: Send + 'static,
    {
        let builder = TonicServer::builder();
        let mut builder = if let (Some(cert_path), Some(key_path)) = (cert_path, key_path) {
            let cert = std::fs::read_to_string(cert_path).expect("invalid TLS cert");
            let key = std::fs::read_to_string(key_path).expect("invalid TLS key");
            let identity = Identity::from_pem(cert, key);
            builder
                .tls_config(ServerTlsConfig::new().identity(identity))
                .expect("invalid TLS configuration")
        } else {
            builder
        }
        .http2_keepalive_interval(Some(Duration::from_secs(5)));

        Self { router: builder.add_service(service), socket: None }
            .add_reflection(tonic::include_file_descriptor_set!("descriptors"))
    }

    pub fn add_service<S>(mut self, service: S) -> Self
    where
        S: Service<Request<Body>, Response = Response<BoxBody>, Error = Infallible>
            + NamedService
            + Clone
            + Send
            + 'static,
        S::Future: Send + 'static,
    {
        self.router = self.router.add_service(service);
        self
    }

    pub fn add_reflection(mut self, encoded_file_descriptor_set: &[u8]) -> Self {
        let reflection_service = tonic_reflection::server::Builder::configure()
            .register_encoded_file_descriptor_set(encoded_file_descriptor_set)
            .build()
            .unwrap();
        self.router = self.router.add_service(reflection_service);
        self
    }

    pub async fn serve(self, endpoint: Endpoint) -> Result<(), ServerError> {
        match endpoint {
            Endpoint::Port(port) => {
                let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port);
                self.router.serve(addr).await?;
            }
            Endpoint::Path(path) => {
                std::fs::create_dir_all(Path::new(&path).parent().unwrap())?;
                // TODO: Have graceful shutdown of server in a higher module clean up the socket.
                // For now, we'll just remove the socket file on creation if it exists already.
                // This will disconnect any existing servers.
                if Path::new(&path).exists() {
                    std::fs::remove_file(&path)?;
                }
                let uds = UnixListener::bind(&path)?;
                let uds_stream = UnixListenerStream::new(uds);
                self.router.serve_with_incoming(uds_stream).await?;
            }
        }
        Ok(())
    }
}
