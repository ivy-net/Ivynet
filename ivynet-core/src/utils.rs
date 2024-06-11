use crate::error::IvyError;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

pub fn read_json<T: for<'a> Deserialize<'a>>(path: PathBuf) -> Result<T, IvyError> {
    let json_str = fs::read_to_string(path)?;
    let res = serde_json::from_str::<T>(&json_str)?;
    Ok(res)
}

pub fn write_json<T>(path: PathBuf, data: &T) -> Result<(), std::io::Error>
where
    T: Serialize,
{
    let json = serde_json::to_string(data)?;
    fs::write(path, json)?;
    Ok(())
}
