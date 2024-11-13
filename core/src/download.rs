use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use std::{cmp::min, path::PathBuf};
use tokio::{
    fs::remove_file,
    io::AsyncWriteExt,
    signal::unix::{signal, SignalKind},
    sync::watch,
};
use tracing::info;

// TODO: Move downloading flow and utils to cli?
// TODO: As this uses a stream, ctrl+c prematurely will lead to a bad file hash. Handle SIGTERM
// correctly.
pub async fn dl_progress_bar(url: &str, file_path: PathBuf) -> Result<(), DownloadError> {
    let res = reqwest::Client::new().get(url).send().await?;
    let size = res.content_length().ok_or(DownloadError::EmptyContent)?;
    let mut file = tokio::fs::File::create(&file_path).await?;

    let pb = ProgressBar::new(size);
    pb.set_style(ProgressStyle::default_bar()
        .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue.width}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")?
        .progress_chars("|>-"));
    pb.set_message(format!("Downloading {}", url));

    let mut downloaded: u64 = 0;
    let mut stream = res.bytes_stream();

    let (tx, rx) = watch::channel(false);

    tokio::spawn(async move {
        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        let mut sigint = signal(SignalKind::interrupt()).unwrap();
        tokio::select! {
            _ = sigterm.recv() => {},
            _ = sigint.recv() => {},
        }
        let _ = tx.send(true);
    });

    while let Some(item) = stream.next().await {
        let chunk = item?;
        file.write_all(&chunk).await?;
        let new = min(downloaded + (chunk.len() as u64), size);
        downloaded = new;
        pb.set_position(new);
        if *rx.borrow() {
            remove_file(file_path).await?;
            info!("sigterm recieved, download canceled");
            return Err(DownloadError::DownloadInt);
        };
    }

    pb.finish_with_message(format!("Downloaded {} to {}", url, file_path.display()));
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("Download interrupted")]
    DownloadInt,
    #[error("Content empty")]
    EmptyContent,
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    IndicatifError(#[from] indicatif::style::TemplateError),
}
