use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use std::{cmp::min, path::PathBuf};
use tokio::io::AsyncWriteExt;

use crate::avs::eigenda::eigenda::CoreError;

pub async fn dl_progress_bar(url: &str, file_path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let res = reqwest::Client::new().get(url).send().await?;
    let size = res.content_length().ok_or(CoreError::DownloadFailed)?;
    let mut file = tokio::fs::File::create(&file_path).await?;

    let pb = ProgressBar::new(size);
    pb.set_style(ProgressStyle::default_bar()
        .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")?
        .progress_chars("|>-"));
    pb.set_message(format!("Downloading {}", url));

    let mut downloaded: u64 = 0;
    let mut stream = res.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item?;
        file.write_all(&chunk).await?;
        let new = min(downloaded + (chunk.len() as u64), size);
        downloaded = new;
        pb.set_position(new);
    }

    pb.finish_with_message(format!("Downloaded {} to {}", url, file_path.display()));
    Ok(())
}
