use std::path::PathBuf;

use ivynet_core::error::IvyError;
use linemux::MuxedLines;

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

pub async fn most_recent_logfile(path: PathBuf) -> Result<PathBuf, IvyError> {
    // get all log files
    let mut files = tokio::fs::read_dir(&path).await?;
    let mut most_recent: Option<String> = None;

    while let Some(file) = files.next_entry().await? {
        let file_name = file.file_name();
        let f = match file_name.to_str() {
            Some(f) => f.to_string(), // Convert to owned String
            None => continue,
        };

        if let Some(ref mr) = most_recent {
            if f > *mr {
                most_recent = Some(f);
            }
        } else {
            most_recent = Some(f);
        }
    }

    match most_recent {
        Some(mr) => Ok(path.join(mr)),
        None => Err(IvyError::NoLogFiles(path.display().to_string())),
    }
}
