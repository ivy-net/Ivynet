use std::{
    convert::Infallible,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};

use tokio::sync::oneshot::Receiver;
use tonic::{
    body::BoxBody,
    codegen::{
        http::{Request, Response},
        Service,
    },
    server::NamedService,
    transport::{server::Router, Body, Identity, Server as TonicServer, ServerTlsConfig},
};

#[derive(Debug, Clone)]
pub enum ServerError {
    UnableToServe,
}

pub struct Server {
    pub router: Router,
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
            builder.tls_config(ServerTlsConfig::new().identity(identity)).expect("invalid TLS configuration")
        } else {
            builder
        }
        .http2_keepalive_interval(Some(Duration::from_secs(5)));

        Self { router: builder.add_service(service) }.add_reflection(tonic::include_file_descriptor_set!("descriptors"))
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

    pub async fn serve(self, port: u16) -> Result<(), ServerError> {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port);
        self.router.serve(addr).await.map_err(|_| ServerError::UnableToServe)?;
        Ok(())
    }

    pub async fn serve_with_shutdown(self, port: u16, shutdown_receiver: Receiver<()>) -> Result<(), ServerError> {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port);
        self.router
            .serve_with_shutdown(addr, async {
                shutdown_receiver.await.ok();
            })
            .await
            .map_err(|_| ServerError::UnableToServe)?;
        Ok(())
    }
}
