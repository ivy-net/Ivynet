/// Basic wrappers around read/write operations. Verbose errors are used as Ivynet relies on
/// frequent path manipulations and file I/O, and the standard error messages are often not
/// descriptive enough.
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
pub enum IoError {
    #[error("File read error: {source} at path {path}")]
    FileReadError {
        #[source]
        source: std::io::Error,
        path: String,
    },

    #[error("File write error: {source} at path {path}")]
    FileWriteError {
        #[source]
        source: std::io::Error,
        path: String,
    },

    #[error("JSON parse error: {source} at path {path}")]
    SerdeJsonError {
        #[source]
        source: serde_json::Error,
        path: String,
    },

    #[error("TOML deserialize error: {source} at path {path}")]
    TomlDeError {
        #[source]
        source: toml::de::Error,
        path: String,
    },

    #[error("TOML serialize error: {source} at path {path}")]
    TomlSerError {
        #[source]
        source: toml::ser::Error,
        path: String,
    },

    #[error("Directory creation error: {source} at path {path}")]
    DirCreationError {
        #[source]
        source: std::io::Error,
        path: String,
    },
}

pub fn read_json<T: for<'a> Deserialize<'a>>(path: &PathBuf) -> Result<T, IoError> {
    let json_str = fs::read_to_string(path)
        .map_err(|e| IoError::FileReadError { source: e, path: path.display().to_string() })?;
    let res = serde_json::from_str::<T>(&json_str)
        .map_err(|e| IoError::SerdeJsonError { source: e, path: path.display().to_string() })?;
    Ok(res)
}

pub fn write_json<T: Serialize>(path: &PathBuf, data: &T) -> Result<(), IoError> {
    let data = serde_json::to_string(data)
        .map_err(|e| IoError::SerdeJsonError { source: e, path: path.display().to_string() })?;
    fs::write(path, data)
        .map_err(|e| IoError::FileWriteError { source: e, path: path.display().to_string() })?;
    Ok(())
}

pub fn read_toml<T: for<'a> Deserialize<'a>>(path: &PathBuf) -> Result<T, IoError> {
    let toml_str = fs::read_to_string(path)
        .map_err(|e| IoError::FileReadError { source: e, path: path.display().to_string() })?;
    let res = toml::from_str(&toml_str)
        .map_err(|e| IoError::TomlDeError { source: e, path: path.display().to_string() })?;
    Ok(res)
}

pub fn write_toml<T: Serialize>(path: &PathBuf, data: &T) -> Result<(), IoError> {
    let data = toml::to_string(data)
        .map_err(|e| IoError::TomlSerError { source: e, path: path.display().to_string() })?;
    fs::write(path, data)
        .map_err(|e| IoError::FileWriteError { source: e, path: path.display().to_string() })?;
    Ok(())
}

pub fn create_dir_all(path: &PathBuf) -> Result<(), IoError> {
    fs::create_dir_all(path)
        .map_err(|e| IoError::DirCreationError { source: e, path: path.display().to_string() })?;
    Ok(())
}

#[cfg(test)]
mod core_io_tests {
    use super::*;
    use std::{fs::File, io::Write};
    use tempfile::tempdir;

    #[test]
    fn test_read_write_json() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");

        let data = vec![1, 2, 3];
        let data_str = serde_json::to_string(&data).unwrap();

        let mut file = File::create(&path).unwrap();
        file.write_all(data_str.as_bytes()).unwrap();

        let res: Vec<i32> = read_json(&path).unwrap();
        assert_eq!(res, data);

        let new_data = vec![4, 5, 6];
        write_json(&path, &new_data).unwrap();

        let res: Vec<i32> = read_json(&path).unwrap();
        assert_eq!(res, new_data);
    }

    #[test]
    fn test_read_write_toml() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.toml");

        let data = vec![1, 2, 3];
        let data_str = toml::to_string(&data).unwrap();

        let mut file = File::create(&path).unwrap();
        file.write_all(data_str.as_bytes()).unwrap();

        let res: Vec<i32> = read_toml(&path).unwrap();
        assert_eq!(res, data);

        let new_data = vec![4, 5, 6];
        write_toml(&path, &new_data).unwrap();

        let res: Vec<i32> = read_toml(&path).unwrap();
        assert_eq!(res, new_data);
    }
}
