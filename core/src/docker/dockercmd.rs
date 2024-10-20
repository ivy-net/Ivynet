use serde::Deserialize;
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    pin::Pin,
    process::Command as BlockingCommand,
    task::{Context, Poll},
};
use tokio::{process::Command, sync::mpsc};
use tokio_stream::Stream;
use tracing::{error, info};

use super::compose_images::{parse_docker_compose_images, ComposeImage};

/// Module for interacting with Docker and Docker Compose.
/// This module provides a wrapper around the `docker-compose` and `docker compose` commands,
/// allowing for easy interaction depending on which is available on the target system.

// TODO: Correctly formatting docker stdout to not interfere with other stdout requires some
// pipe management and must be tested with respect to docker's logging behavior.

#[derive(Debug, Clone, Deserialize)]
pub struct ImageExposedPort {
    #[serde(rename = "HostIp")]
    pub ip: String,
    #[serde(rename = "HostPort")]
    pub port: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NetworkSettings {
    #[serde(rename = "Ports")]
    pub ports: HashMap<String, Vec<ImageExposedPort>>,
}

#[derive(Debug, Deserialize)]
pub struct ImageDetails {
    #[serde(rename = "Image")]
    pub image: String,

    #[serde(rename = "NetworkSettings")]
    pub network_settings: NetworkSettings,
}

/// Wrapper struct for commands targeting docker-compose files. Initialization targets etiher
/// `docker-compose` or `docker compose` depending on availability.
#[derive(Debug)]
pub struct DockerCmd {
    cmd: Command,
    args: Vec<String>,
    current_dir: Option<PathBuf>,
}

/// Handle to a running docker-compose service. This handle will automatically stop the service
/// when dropped. The service is stopped by running `docker-compose -f <filename> down` in the
/// directory where the service was started. If the struct is dropped before the container has
/// finished starting, the container will not be brought down and will result in a dangling
/// container.
#[derive(Debug)]
pub struct DockerChild {
    pub run_path: PathBuf,
    pub filename: String,
    pub handle: tokio::process::Child,
    down_on_drop: bool,
}

impl Deref for DockerCmd {
    type Target = Command;

    fn deref(&self) -> &Self::Target {
        &self.cmd
    }
}

impl DerefMut for DockerCmd {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.cmd
    }
}

impl DockerCmd {
    pub fn new() -> Self {
        let cmd = which_dockercmd();
        Self { cmd, args: Vec::new(), current_dir: None }
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S> + Clone,
        S: AsRef<std::ffi::OsStr>,
    {
        self.args =
            args.clone().into_iter().map(|s| s.as_ref().to_string_lossy().to_string()).collect();
        self.cmd.args(args);
        self
    }

    pub fn current_dir<P: AsRef<Path>>(mut self, path: P) -> Self {
        let path = path.as_ref().to_path_buf();
        self.current_dir = Some(path.clone());
        self.cmd.current_dir(path);
        self
    }

    pub async fn spawn_dockerchild(mut self) -> Result<DockerChild, std::io::Error> {
        let run_path =
            self.current_dir.clone().unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        let filename = self.extract_args_filename();
        let handle = self.spawn()?;
        Ok(DockerChild::new(run_path, filename, handle))
    }

    fn extract_args_filename(&self) -> String {
        let mut filename = None;
        let mut args = self.args.iter();
        while let Some(arg) = args.next() {
            if arg == "-f" {
                filename = args.next().map(|s| s.to_string());
                break;
            }
        }
        filename.unwrap_or_else(|| "docker-compose.yml".to_string())
    }
}

impl Default for DockerCmd {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for DockerChild {
    fn drop(&mut self) {
        if self.down_on_drop {
            self.down();
        }
    }
}

impl DockerChild {
    pub fn new(run_path: PathBuf, filename: String, handle: tokio::process::Child) -> Self {
        Self { run_path, filename, handle, down_on_drop: true }
    }

    /// Get the images of the running docker-compose service.
    pub async fn images(&self) -> Result<Vec<ComposeImage>, std::io::Error> {
        let mut images = Vec::new();
        let output = DockerCmd::new()
            .current_dir(&self.run_path)
            .args(["-f", &self.filename, "images"])
            .output()
            .await?;
        let output = parse_docker_compose_images(&output.stdout);
        Ok(images)
    }

    /// Bring down the docker-compose service.
    pub fn down(&self) {
        let mut cmd = which_dockercmd_blocking();
        let status =
            cmd.args(["-f", &self.filename]).current_dir(&self.run_path).arg("down").output();
        match status {
            Ok(output) => {
                // stderr to string
                let msg = std::str::from_utf8(&output.stderr).unwrap();
                info!("Docker down status: {:?}", msg);
            }
            Err(e) => {
                error!("Docker down error: {:?}", e);
            }
        }
    }

    /// Set whether the container should be brought down when the struct is dropped.
    pub fn down_on_drop(&mut self, down_on_drop: bool) {
        self.down_on_drop = down_on_drop;
    }
}

pub async fn inspect(image_name: &str) -> Option<ImageDetails> {
    if let Ok(output) = Command::new("docker").arg("inspect").arg(image_name).output().await {
        match serde_json::from_str::<Vec<ImageDetails>>(
            std::str::from_utf8(&output.stdout).expect("Unparsable output string"),
        ) {
            Ok(command_result) => return command_result.into_iter().next(),
            Err(e) => error!("Parse inspection error {e:?}"),
        }
    }
    None
}

pub struct DockerStream(mpsc::UnboundedReceiver<(String, bool)>);

impl Stream for DockerStream {
    type Item = (String, bool);

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let inner = self.get_mut();
        inner.0.poll_recv(cx)
    }
}

#[allow(dead_code)]
/// Returns an async command for docker-compose or docker compose.
fn which_dockercmd() -> Command {
    let cmd = if which::which("docker-compose").is_ok() {
        Command::new("docker-compose")
    } else {
        let mut cmd = Command::new("docker");
        cmd.arg("compose");
        cmd
    };
    cmd
}

/// Returns a blocking command for docker-compose or docker compose.
fn which_dockercmd_blocking() -> BlockingCommand {
    if which::which("docker-compose").is_ok() {
        BlockingCommand::new("docker-compose")
    } else {
        let mut cmd = BlockingCommand::new("docker");
        cmd.arg("compose");
        cmd
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{thread::sleep, time::Duration};

    fn test_compose_dir() -> PathBuf {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("src/docker/test");
        path
    }

    #[tokio::test]
    async fn test_dockercmd_status() {
        let test_dir = test_compose_dir();
        let status = DockerCmd::new()
            .current_dir(&test_dir)
            .args(["-f", "counter-test-compose.yml", "up"])
            .status()
            .await
            .unwrap();
        assert!(status.success());
    }

    #[tokio::test]
    async fn test_dockerchild_images() {
        let test_dir = test_compose_dir().join("counter");
        let child = DockerCmd::new()
            .current_dir(&test_dir)
            .args(["-f", "counter-test-compose.yml", "up", "-d"])
            .spawn_dockerchild()
            .await
            .unwrap();

        // wait for container startup
        sleep(Duration::from_secs(5));

        let images = child.images().await.unwrap();
        println!("Images: {:?}", images);
    }
}
