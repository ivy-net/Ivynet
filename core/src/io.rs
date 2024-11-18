/// Basic wrappers around read/write operations. Verbose errors are used as Ivynet relies on
/// frequent path manipulations and file I/O, and the standard error messages are often not
/// descriptive enough.
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;
use zip::read::ZipFile;

#[derive(ThisError, Debug)]
pub enum IoError {
    #[error("File read error at path {path}")]
    FileReadError {
        #[source]
        source: std::io::Error,
        path: String,
    },

    #[error("File write error at path {path}")]
    FileWriteError {
        #[source]
        source: std::io::Error,
        path: String,
    },

    #[error("JSON parse error at path {path}")]
    SerdeJsonError {
        #[source]
        source: serde_json::Error,
        path: String,
    },

    #[error("TOML deserialize error at path {path}")]
    TomlDeError {
        #[source]
        source: toml::de::Error,
        path: String,
    },

    #[error("TOML serialize error at path {path}")]
    TomlSerError {
        #[source]
        source: toml::ser::Error,
        path: String,
    },

    #[error("YAML serialize error at path {path}")]
    YamlSerError {
        #[source]
        source: serde_yaml::Error,
        path: String,
    },

    #[error("YAML deserialize error at path {path}")]
    YamlDeError {
        #[source]
        source: serde_yaml::Error,
        path: String,
    },

    #[error("Directory creation error at path {path}")]
    DirCreationError {
        #[source]
        source: std::io::Error,
        path: String,
    },
    #[error(transparent)]
    ZipFileError(#[from] zip::result::ZipError),
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

pub fn read_yaml<T: for<'a> Deserialize<'a>>(path: &PathBuf) -> Result<T, IoError> {
    let yaml_str = fs::read_to_string(path)
        .map_err(|e| IoError::FileReadError { source: e, path: path.display().to_string() })?;
    let res = serde_yaml::from_str::<T>(&yaml_str)
        .map_err(|e| IoError::YamlDeError { source: e, path: path.display().to_string() })?;
    Ok(res)
}

pub fn write_yaml<T: Serialize>(path: &PathBuf, data: &T) -> Result<(), IoError> {
    let data = serde_yaml::to_string(data)
        .map_err(|e| IoError::YamlSerError { source: e, path: path.display().to_string() })?;
    fs::write(path, data)
        .map_err(|e| IoError::FileWriteError { source: e, path: path.display().to_string() })?;
    Ok(())
}

pub fn create_dir_all(path: &PathBuf) -> Result<(), IoError> {
    fs::create_dir_all(path)
        .map_err(|e| IoError::DirCreationError { source: e, path: path.display().to_string() })?;
    Ok(())
}

/// Unzips `zipfile` to `to` directory, depositing contents of `zipfile` directly to `to`.
pub fn unzip_to(zip_file: &PathBuf, to: &Path) -> Result<(), IoError> {
    if zip_file.extension().unwrap() != "zip" {
        return Err(IoError::FileReadError {
            source: std::io::Error::new(std::io::ErrorKind::InvalidInput, "Not a zip file"),
            path: zip_file.display().to_string(),
        });
    }
    let zip_file = fs::File::open(zip_file)
        .map_err(|e| IoError::FileReadError { source: e, path: zip_file.display().to_string() })?;
    let mut archive = zip::ZipArchive::new(zip_file)?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = match file.enclosed_name() {
            Some(path) => to.join(path),
            None => continue,
        };

        {
            let comment = file.comment();
            if !comment.is_empty() {
                println!("File {i} comment: {comment}");
            }
        }

        if file.is_dir() {
            fs::create_dir_all(&outpath).map_err(|e| IoError::DirCreationError {
                source: e,
                path: outpath.display().to_string(),
            })?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p).map_err(|e| IoError::DirCreationError {
                        source: e,
                        path: p.display().to_string(),
                    })?;
                }
            }
            let mut outfile = fs::File::create(&outpath).map_err(|e| IoError::FileWriteError {
                source: e,
                path: outpath.display().to_string(),
            })?;
            std::io::copy(&mut file, &mut outfile).map_err(|e| IoError::FileWriteError {
                source: e,
                path: outpath.display().to_string(),
            })?;
        }

        // Get and Set permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&outpath, fs::Permissions::from_mode(mode)).unwrap();
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod core_io_tests {
    use super::*;
    use std::{fs::File, io::Write};
    use tempfile::tempdir;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct TestStruct {
        a: i32,
        b: i32,
        c: i32,
    }

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
        let data = TestStruct { a: 1, b: 2, c: 3 };

        let dir = tempdir().unwrap();
        let path = dir.path().join("test.toml");

        write_toml(&path, &data).unwrap();

        let res: TestStruct = read_toml(&path).unwrap();
        assert_eq!(res, data);
    }
}
