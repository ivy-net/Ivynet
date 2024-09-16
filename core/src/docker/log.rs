use std::{
    fs::{self, File, OpenOptions},
    path::PathBuf,
};

use crate::error::IvyError;

pub enum CmdLogSource {
    StdOut,
    StdErr,
}

pub fn open_logfile(logfile: &PathBuf) -> Result<File, IvyError> {
    let parent = logfile.parent().unwrap();
    fs::create_dir_all(parent)?;
    let file = OpenOptions::new().create(true).append(true).open(logfile)?;
    Ok(file)
}
