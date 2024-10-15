use serde::Deserialize;
use std::{
    collections::HashMap,
    ffi::OsStr,
    ops::{Deref, DerefMut},
    path::PathBuf,
    pin::Pin,
    process::ExitStatus,
    task::{Context, Poll},
};
use tokio::{
    process::{Child, Command},
    sync::mpsc,
};
use tokio_stream::Stream;
use tracing::error;

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

pub async fn docker_cmd<I, S>(args: I) -> Result<Child, std::io::Error>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut cmd = if which::which("docker-compose").is_ok() {
        Command::new("docker-compose")
    } else {
        let mut cmd = Command::new("docker");
        cmd.arg("compose");
        cmd
    };
    cmd.args(args).spawn()
}

pub async fn docker_cmd_status<I, S>(
    args: I,
    dir: Option<PathBuf>,
) -> Result<ExitStatus, std::io::Error>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut cmd = if which::which("docker-compose").is_ok() {
        Command::new("docker-compose")
    } else {
        let mut cmd = Command::new("docker");
        cmd.arg("compose");
        cmd
    };
    if let Some(dir) = dir {
        cmd.current_dir(dir);
    }
    cmd.args(args).status().await
}

pub struct DockerStream(mpsc::UnboundedReceiver<(String, bool)>);

impl Stream for DockerStream {
    type Item = (String, bool);

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let inner = self.get_mut();
        inner.0.poll_recv(cx)
    }
}

pub async fn inspect(image_name: &str) -> Option<ImageDetails> {
    if let Some(output) = Command::new("docker").arg("inspect").arg(image_name).output().await.ok()
    {
        match serde_json::from_str::<Vec<ImageDetails>>(
            std::str::from_utf8(&output.stdout).expect("Unparsable output string"),
        ) {
            Ok(command_result) => return command_result.into_iter().next(),
            Err(e) => error!("Parse inspection error {e:?}"),
        }
    }
    None
}
pub struct DockerCmd(Command);

impl Deref for DockerCmd {
    type Target = Command;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DockerCmd {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl DockerCmd {
    pub fn new() -> Self {
        let cmd = if which::which("docker-compose").is_ok() {
            Command::new("docker-compose")
        } else {
            let mut cmd = Command::new("docker");
            cmd.arg("compose");
            cmd
        };
        Self(cmd)
    }
}

impl Default for DockerCmd {
    fn default() -> Self {
        Self::new()
    }
}
