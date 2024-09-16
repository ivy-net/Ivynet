use std::path::PathBuf;

use ivynet_core::error::IvyError;
use linemux::MuxedLines;

/// Tail logs of the loaded AVS
pub async fn tail_logs(path: PathBuf, _num_lines: u32) -> Result<(), IvyError> {
    let mut lines = MuxedLines::new()?;
    lines.add_file_from_start(path).await?;
    while let Ok(Some(line)) = lines.next_line().await {
        println!("{}", line.line());
    }
    Ok(())
}
