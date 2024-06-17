use crate::error::IvyError;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tracing::debug;

pub fn read_json<T: for<'a> Deserialize<'a>>(path: PathBuf) -> Result<T, IvyError> {
    let json_str = fs::read_to_string(path)?;
    let res = serde_json::from_str::<T>(&json_str)?;
    Ok(res)
}

pub fn write_json<T: Serialize>(path: PathBuf, data: &T) -> Result<(), IvyError> {
    let data = serde_json::to_string(data)?;
    debug!("json write: {}", path.display());
    fs::write(path, data)?;
    Ok(())
}

pub fn read_toml<T: for<'a> Deserialize<'a>>(path: PathBuf) -> Result<T, IvyError> {
    let toml_str = fs::read_to_string(path)?;
    let res = toml::from_str(&toml_str)?;
    Ok(res)
}

pub fn write_toml<T: Serialize>(path: PathBuf, data: &T) -> Result<(), IvyError> {
    let data = toml::to_string(data)?;
    debug!("toml write: {}", path.display());
    fs::write(path, data)?;
    Ok(())
}
