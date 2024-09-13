use std::{
    ffi::OsStr,
    pin::Pin,
    process::{ExitStatus, Stdio},
    task::{Context, Poll},
};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
    sync::mpsc,
    task,
};
use tokio_stream::Stream;

use crate::error::IvyError;

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

    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()
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

pub async fn stream_docker_output(child: &mut Child) -> Result<DockerStream, IvyError> {
    // TODO: Replace these expects with an if let Some() { } else { } block.
    let stdout = child.stdout.take().expect("failed to capture stdout");
    let stderr = child.stderr.take().expect("failed to capture stderr");

    let (tx, rx) = mpsc::unbounded_channel();

    task::spawn(stream_output(stdout, tx.clone(), false));
    task::spawn(stream_output(stderr, tx, true));
    Ok(DockerStream(rx))
}

async fn stream_output<R>(reader: R, tx: mpsc::UnboundedSender<(String, bool)>, is_stderr: bool)
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut lines = BufReader::new(reader).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        if tx.send((line, is_stderr)).is_err() {
            println!("Failed to send line to channel, breaking channel.");
            break;
        }
    }
}
