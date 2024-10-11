use serde::Deserialize;
use std::{
    collections::HashMap,
    ffi::OsStr,
    pin::Pin,
    process::ExitStatus,
    task::{Context, Poll},
};
use tokio::{
    process::{Child, Command},
    sync::mpsc,
};
use tokio_stream::Stream;

#[derive(Debug, Clone, Deserialize)]
pub struct ImageExposedPort {
    #[serde(rename = "HostIp")]
    pub ip: String,
    #[serde(rename = "HostPort")]
    pub port: u32,
}

#[derive(Debug, Deserialize)]
pub struct ImageDetails {
    #[serde(rename = "NetworkSettings")]
    pub network_settings: HashMap<String, ImageExposedPort>,

    #[serde(rename = "Image")]
    pub image: String,
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

pub async fn docker_cmd_status<I, S>(args: I) -> Result<ExitStatus, std::io::Error>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    if which::which("docker-compose").is_ok() {
        Command::new("docker-compose").args(args).status().await
    } else {
        Command::new("docker").arg("compose").args(args).status().await
    }
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
        if let Some(command_result) = serde_json::from_str::<Vec<ImageDetails>>(
            std::str::from_utf8(&output.stdout).expect("Unparsable output string"),
        )
        .ok()
        {
            return command_result.into_iter().next();
        }
    }
    None
}
