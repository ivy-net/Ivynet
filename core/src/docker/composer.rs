use std::path::PathBuf;

use tokio::process::Command;
use tracing::{error, warn};

pub struct DockerInstanceBuilder {
    path: String,
}

impl DockerInstanceBuilder {
    fn new(path: String) -> Self {
        Self { path }
    }
    async fn swarm(self) -> Result<DockerSwarmHandle, DockerSwarmError> {
        let path = std::path::PathBuf::from(self.path);
        let swarm_is_active = Command::new("docker").args(["swarm", "ca"]).output().await?;
        if swarm_is_active.status.success() {
            warn!("Swarm already initialized, if you want to reinitialize the swarm, please run `docker swarm leave --force.` Continuing with the existing swarm. This warning can be safely ignored.");
        } else {
            let swarm_init = Command::new("docker")
                .args(["swarm", "init", "--advertise-addr", "127.0.0.1"])
                .output()
                .await?;
            if !swarm_init.status.success() {
                error!("Swarm initialization failed: {:#?}", swarm_init);
            }
        }
        Ok(DockerSwarmHandle::new(path))
    }
    fn compose(self) -> Result<DockerComposeHandle, DockerError> {
        let path = std::fs::canonicalize(std::path::PathBuf::from(self.path))?;
        Ok(DockerComposeHandle::new(path))
    }
}

#[tokio::test]
async fn test_swarm() {
    let builder = DockerInstanceBuilder::new("./fluentd/fluentd-compose.yaml".to_string());
    let swarm = builder.swarm().await.unwrap();
    println!("Swarm name: {}", swarm.name());
    swarm.up().unwrap();
    swarm.ls().unwrap();
    swarm.set_logging_driver("fluentd", "fluentd-address=localhost:24224").unwrap();
}

pub struct DockerSwarmHandle {
    path: std::path::PathBuf,
}

impl DockerSwarmHandle {
    fn new(path: std::path::PathBuf) -> Self {
        Self { path }
    }
    fn name(&self) -> String {
        self.path.file_stem().unwrap().to_str().unwrap().to_string()
    }
    fn up(&self) -> Result<(), DockerSwarmError> {
        println!("pwd: {:?}", std::env::current_dir()?);
        println!("target exists: {:?}", self.path.exists());
        let output = std::process::Command::new("docker")
            .args(["stack", "deploy", "-c", &self.path.display().to_string(), &self.name()])
            .output()?;
        println!("{:#?}", output);
        if !output.status.success() {
            return Err(DockerSwarmError::SwarmInitFailed);
        }
        Ok(())
    }
    fn ls(&self) -> Result<Vec<DockerService>, DockerSwarmError> {
        let output = std::process::Command::new("docker").args(["service", "ls"]).output()?;
        let services = parse_docker_service_ls(&String::from_utf8_lossy(&output.stdout));
        println!("Services: {:#?}", services);
        Ok(services)
    }
    fn set_logging_driver(&self, driver: &str, opt: &str) -> Result<(), DockerSwarmError> {
        println!("{}", self.name());
        let services = self.ls()?;
        for service in services {
            println!("Updating service: {}", service.name);
            let output = std::process::Command::new("docker")
                .args([
                    "service",
                    "update",
                    "--log-driver",
                    driver,
                    "--log-opt",
                    opt,
                    &service.name,
                ])
                .output()?;
            println!("{:#?}", output);
            if !output.status.success() {
                return Err(DockerSwarmError::SetLoggingDriverFailed);
            }
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DockerSwarmError {
    #[error("Swarm already initialized")]
    SwarmAlreadyInitialized,

    #[error("Swarm initialization failed")]
    SwarmInitFailed,

    #[error("Set logging driver failed")]
    SetLoggingDriverFailed,

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub struct DockerComposeHandle {
    path: std::path::PathBuf,
}

impl DockerComposeHandle {
    pub fn new(compose_file: PathBuf) -> Self {
        Self { path: compose_file }
    }
    //
    //     pub fn up(&self) -> Result<(), DockerComposeError> {
    //         let child = std::process::Command::new("docker-compose")
    //             .args(["-f", &self.path.display().to_string(), "up", "-d"])
    //             .spawn()?;
    //         Ok(())
    //     }
    //
    //     pub fn down(&self) -> Result<(), DockerComposeError> {
    //         let output = std::process::Command::new("docker-compose")
    //             .args(["-f", &self.path.display().to_string(), "down"])
    //             .output()?;
    //         if !output.status.success() {
    //             return Err(DockerComposeError::ComposeDownFailed);
    //         }
    //         Ok(())
    //     }
    //
    //     pub fn ps(&self) -> Result<(), DockerComposeError> {
    //         let output = std::process::Command::new("docker-compose")
    //             .args(["-f", &self.path.display().to_string(), "ps"])
    //             .output()?;
    //         if !output.status.success() {
    //             return Err(DockerComposeError::ComposePsFailed);
    //         }
    //         let services = parse_compose_ps_output(&String::from_utf8_lossy(&output.stdout));
    //         println!("Services: {:#?}", services);
    //         Ok(())
    //     }
}

/// Parsed output of `docker service ls`
#[derive(Debug)]
struct DockerService {
    id: String,
    name: String,
    mode: String,
    replicas: String,
    image: String,
    ports: String,
}

fn parse_docker_service_ls(output: &str) -> Vec<DockerService> {
    let mut lines = output.lines();

    // Parse header to get column positions
    let header = lines.next().expect("Header line missing");
    let columns = ["ID", "NAME", "MODE", "REPLICAS", "IMAGE", "PORTS"];
    let positions: Vec<_> = columns
        .iter()
        .map(|&col| header.find(col).expect(&format!("Column '{}' not found", col)))
        .collect();

    lines
        .filter_map(|line| {
            if line.trim().is_empty() {
                return None;
            }

            let mut service = DockerService {
                id: String::new(),
                name: String::new(),
                mode: String::new(),
                replicas: String::new(),
                image: String::new(),
                ports: String::new(),
            };

            for i in 0..positions.len() {
                let start = positions[i];
                let end = positions.get(i + 1).copied().unwrap_or(line.len());
                let value = line[start..end].trim();

                match i {
                    0 => service.id = value.to_string(),
                    1 => service.name = value.to_string(),
                    2 => service.image = value.to_string(),
                    3 => service.mode = value.to_string(),
                    4 => service.replicas = value.to_string(),
                    5 => service.ports = value.to_string(),
                    _ => {}
                }
            }

            Some(service)
        })
        .collect()
}

#[derive(Debug, thiserror::Error)]
pub enum DockerError {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("docker-compose up failed")]
    ComposeUpFailed,

    #[error("docker-compose down failed")]
    ComposeDownFailed,

    #[error("docker-compose ps failed")]
    ComposePsFailed,
}
