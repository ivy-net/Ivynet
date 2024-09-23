use std::{env, fs::File, io::Write as _, path::PathBuf, process::Command};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let version_file_path = out.join("version.rs");

    let output =
        Command::new("git").args(vec!["log", "-n 1", r#"--pretty=format:"%H""#]).output()?;
    let hash = std::str::from_utf8(&output.stdout)?.replace("\"", "");

    let mut version_file = File::create(version_file_path)?;

    let version_hash = format!("{} ({hash})", env!("CARGO_PKG_VERSION"));
    version_file
        .write_all(format!(r#"pub const VERSION_HASH: &str = "{version_hash}";"#).as_bytes())?;

    version_file.sync_all()?;
    Ok(())
}
