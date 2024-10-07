use std::{
    ffi::OsStr,
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
