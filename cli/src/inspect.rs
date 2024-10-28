use futures::future::BoxFuture;
use ivynet_core::error::IvyError;
use linemux::MuxedLines;
use std::path::PathBuf;

/// Tail logs of the loaded AVS
pub async fn tail_logs(path: PathBuf, _num_lines: u32) -> Result<(), IvyError> {
    println!("{:#?}", path);
    let mut lines = MuxedLines::new()?;
    lines.add_file_from_start(path).await?;
    while let Ok(Some(line)) = lines.next_line().await {
        println!("{}", line.line());
    }
    Ok(())
}

pub fn select_logfile(
    path: PathBuf,
    mut depth: u8,
) -> BoxFuture<'static, Result<PathBuf, IvyError>> {
    Box::pin(async move {
        let mut available_paths = Vec::new();
        let mut files = tokio::fs::read_dir(&path).await?;

        while let Some(file) = files.next_entry().await? {
            let file_name = file.file_name();
            let f = match file_name.to_str() {
                Some(f) => f.to_string(),
                None => continue,
            };

            if f.starts_with("$") {
                continue;
            }

            if file.metadata().await?.is_dir() || f.ends_with(".log") {
                available_paths.push(f);
            }
        }

        // Sort in descending order for date-based logs

        available_paths.sort();
        available_paths.reverse();

        // Show prompt at depth 0 only
        let selection = if depth == 0 {
            dialoguer::Select::new()
                .with_prompt(
                    "Select a log directory to inspect. This will traverse the log file tree",
                )
                .items(&available_paths)
                .default(0)
                .interact()?
        } else {
            dialoguer::Select::new().items(&available_paths).default(0).interact()?
        };

        let new_path = path.join(available_paths[selection].clone());

        if new_path.is_dir() {
            depth += 1;
            select_logfile(new_path, depth).await
        } else {
            Ok(new_path)
        }
    })
}
