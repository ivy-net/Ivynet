use ivynet_core::{
    avs::build_avs_provider,
    config::IvyConfig,
    grpc::{
        backend::backend_client::BackendClient,
        client::{create_channel, Uri},
        ivynet_api::{
            ivy_daemon_avs::avs_server::AvsServer,
            ivy_daemon_operator::operator_server::OperatorServer,
        },
        server::{Endpoint, Server},
    },
    wallet::IvyWallet,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::error;

use crate::{error::Error, rpc::ivynet::IvynetService, telemetry};

pub async fn serve(
    avs: Option<String>,
    chain: Option<String>,
    config: &IvyConfig,
    keyfile_pw: &str,
    server_url: Uri,
    server_ca: Option<String>,
    no_backend: bool,
) -> Result<(), Error> {
    let sock = Endpoint::Path(config.uds_dir());

    // Keystore load
    let wallet = IvyWallet::from_keystore(config.default_ecdsa_keyfile.clone(), keyfile_pw)?;

    // Avs Service
    // TODO: This should default to local instead of holesky?
    let chain = chain.unwrap_or_else(|| "holesky".to_string());
    let avs_provider = build_avs_provider(
        avs.as_deref(),
        &chain,
        config,
        Some(wallet.clone()),
        Some(keyfile_pw.to_owned()),
    )
    .await?;
    let ivynet_inner = Arc::new(RwLock::new(avs_provider));

    // NOTE: Due to limitations with Prost / GRPC, we create a new server with a reference-counted
    // handle to the inner type for each server, as opposed to cloning / being able to clone the
    // outer service.
    let avs_server = AvsServer::new(IvynetService::new(ivynet_inner.clone()));
    let operator_server = OperatorServer::new(IvynetService::new(ivynet_inner.clone()));
    let backend_client = BackendClient::new(
        create_channel(ivynet_core::grpc::client::Source::Uri(server_url), server_ca).await?,
    );

    let server = Server::new(avs_server, None, None).add_service(operator_server);

    // Logging service
    println!("Starting the IvyNet logging service...");
    // TODO: Try running from different directory, ensure this is project path relative
    let fluentd_path = "./fluentd/fluentd-compose.yaml";
    let fluentd_compose = DockerComposeHandle::new(fluentd_path)?;
    fluentd_compose.up()?;
    fluentd_compose.ps()?;

    println!("Starting the IvyNet service at {}...", sock);

    if no_backend {
        server.serve(sock).await?;
    } else {
        let connection_wallet = config.identity_wallet()?;
        tokio::select! {
            ret = server.serve(sock) => { error!("Local server error {ret:?}") },
            ret = telemetry::listen(ivynet_inner, backend_client, connection_wallet) => { error!("Telemetry listener error {ret:?}") }
        }
    }

    Ok(())
}

pub struct DockerComposeHandle {
    path: std::path::PathBuf,
}

impl DockerComposeHandle {
    pub fn new(compose_file: &str) -> Result<Self, DockerComposeError> {
        println!("Compose file: {}", compose_file);
        let path = std::fs::canonicalize(std::path::PathBuf::from(compose_file))?;
        println!("Compose file path: {:?}", path);
        Ok(Self { path })
    }

    pub fn up(&self) -> Result<(), DockerComposeError> {
        let child = std::process::Command::new("docker-compose")
            .args(["-f", &self.path.display().to_string(), "up", "-d"])
            .spawn()?;
        Ok(())
    }

    pub fn down(&self) -> Result<(), DockerComposeError> {
        let output = std::process::Command::new("docker-compose")
            .args(["-f", &self.path.display().to_string(), "down"])
            .output()?;
        if !output.status.success() {
            return Err(DockerComposeError::ComposeDownFailed);
        }
        Ok(())
    }

    pub fn ps(&self) -> Result<(), DockerComposeError> {
        let output = std::process::Command::new("docker-compose")
            .args(["-f", &self.path.display().to_string(), "ps"])
            .output()?;
        if !output.status.success() {
            return Err(DockerComposeError::ComposePsFailed);
        }
        let services = parse_compose_ps_output(&String::from_utf8_lossy(&output.stdout));
        println!("Services: {:#?}", services);
        Ok(())
    }
}

/// Parser for docker-compose ps output
fn parse_compose_ps_output(output: &str) -> Vec<DockerService> {
    let mut services = Vec::new();
    let lines: Vec<&str> = output.lines().skip(1).collect(); // Skip header line

    for line in lines {
        let mut parts = Vec::new();
        let mut current_part = String::new();
        let mut in_quotes = false;

        for c in line.chars() {
            match c {
                '"' => {
                    in_quotes = !in_quotes;
                    current_part.push(c);
                }
                ' ' if !in_quotes => {
                    if !current_part.is_empty() {
                        parts.push(current_part.trim().to_string());
                        current_part.clear();
                    }
                }
                _ => current_part.push(c),
            }
        }

        let name = parts[0].to_string();
        let image = parts[1].to_string();
        let command = parts[2].trim_matches('"').to_string();
        let service = parts[3].to_string();
        let created = format!("{} {} {}", parts[4], parts[5], parts[6]);

        services.push(DockerService { name, image, command, service, created });
    }

    services
}

#[derive(Debug, PartialEq, Clone)]
pub struct DockerService {
    name: String,
    image: String,
    command: String,
    service: String,
    created: String,
}

#[derive(Debug, thiserror::Error)]
pub enum DockerComposeError {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("docker-compose up failed")]
    ComposeUpFailed,

    #[error("docker-compose down failed")]
    ComposeDownFailed,

    #[error("docker-compose ps failed")]
    ComposePsFailed,
}

#[test]
fn test_parse_ps_output() {
    let output = r#"NAME            IMAGE     COMMAND           SERVICE    CREATED         STATUS          PORTS
example-foo-1   alpine    "docker-entrypoint.s…"   foo        4 seconds ago   Up 2 seconds    0.0.0.0:8080->80/tcp
example-bar-1   alpine    "docker-entrypoint.s…"   bar        4 seconds ago   exited (0)"#;
    let services = parse_compose_ps_output(output);
    let result_service_1 = DockerService {
        name: "example-foo-1".to_string(),
        image: "alpine".to_string(),
        command: "docker-entrypoint.s…".to_string(),
        service: "foo".to_string(),
        created: "4 seconds ago".to_string(),
    };
    let result_service_2 = DockerService {
        name: "example-bar-1".to_string(),
        image: "alpine".to_string(),
        command: "docker-entrypoint.s…".to_string(),
        service: "bar".to_string(),
        created: "4 seconds ago".to_string(),
    };
    assert_eq!(services[0], result_service_1);
    assert_eq!(services[1], result_service_2);
}
