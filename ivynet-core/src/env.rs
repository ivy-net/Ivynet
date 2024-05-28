use std::{collections::HashMap, fs};

use tracing::debug;

pub fn edit_env_vars(filename: &str, env_values: HashMap<&str, &str>) -> Result<(), Box<dyn std::error::Error>> {
    debug!("{:?}", env_values);
    let contents = fs::read_to_string(filename)?;
    let new_contents = contents
        .lines()
        .map(|line| {
            let mut parts = line.splitn(2, '=');
            let key: &str = parts.next().unwrap();
            if let Some(value) = env_values.get(key) {
                format!("{}={}", key, value)
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(filename, new_contents.as_bytes())?;
    debug!("writing env vars: {}", filename);
    Ok(())
}
