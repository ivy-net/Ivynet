use std::{
    fs::{self, File, OpenOptions},
    path::PathBuf,
};

use crate::error::IvyError;

pub enum CmdLogSource {
    StdOut,
    StdErr,
}

pub enum CmdLogType {
    Error,
    Warn,
    Info,
    Debug,
}

pub fn split_log_to_container(log: &str) -> (&str, &str) {
    let (container, log) = log.split_once('|').unwrap_or(("outer", log));
    (container.trim(), log.trim())
}

pub fn open_logfile(logfile: &PathBuf) -> Result<File, IvyError> {
    let parent = logfile.parent().unwrap();
    fs::create_dir_all(parent)?;

    let file = OpenOptions::new().create(true).append(true).open(logfile)?;
    Ok(file)
}

#[derive(Debug, PartialEq)]
struct EigendaNativeNodeLog {}
