use std::process::Stdio;

use tokio::process::Command;

// A wrapper around the Eigenlayer CLI. Currently unused.

const EIGENLAYER_SOURCE: &str = "https://raw.githubusercontent.com/layr-labs/eigenlayer-cli/master/scripts/install.sh";

// TODO: This is only tested for Linux. This will almost certainly return an incorrect
// path for Windows systems.
pub async fn setup_eigenlayer_cli_binary() -> Result<String, Box<dyn std::error::Error>> {
    let install = reqwest::get(EIGENLAYER_SOURCE).await?.text().await?;
    let output = Command::new("sh").arg("-c").arg(&install).stdout(Stdio::piped()).output().await?;
    let output_str = String::from_utf8(output.stdout)?;
    println!("{}", output_str);
    Ok("~/bin/eigenlayer".to_string())
}

pub async fn generate_bls_keypair(
    eigenlayer_path: &str,
    password: &str,
    keyname: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("sh")
        .arg(eigenlayer_path)
        .arg(format!("echo {} |", password))
        .arg("operator")
        .arg("keys")
        .arg("create --key-type bls")
        .arg(keyname)
        .stdout(Stdio::piped())
        .output()
        .await?;
    let output_str = String::from_utf8(output.stdout)?;
    println!("{}", output_str);
    Ok(format!("{}.bls.key.json", keyname))
}

/// A wrapper around an Eigenlayer CLI binary with convenience functions.
pub struct EigenlayerCli {
    path: String,
}

impl EigenlayerCli {
    /// Intializes the Eigenlayer CLI from a path.
    pub fn from_path(path: &str) -> Self {
        Self { path: path.to_string() }
    }

    /// Pulls the eigenlayer CLI binary from remote and installs it to ~/bin/eigenlayer on the
    /// local system
    pub async fn install_from_remote() -> Result<Self, Box<dyn std::error::Error>> {
        let install = reqwest::get(EIGENLAYER_SOURCE).await?.text().await?;
        let output = Command::new("sh").arg("-c").arg(&install).stdout(Stdio::piped()).output().await?;
        let output_str = String::from_utf8(output.stdout)?;
        println!("{}", output_str);
        Ok(Self { path: "~/bin/eigenlayer".to_string() })
    }

    pub async fn generate_bls_keypair(
        &self,
        keyname: &str,
        password: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let output = Command::new("sh")
            .arg(&self.path)
            .arg(format!("echo {} |", password))
            .arg("operator")
            .arg("keys")
            .arg("create --key-type bls")
            .arg(keyname)
            .stdout(Stdio::piped())
            .output()
            .await?;
        let output_str = String::from_utf8(output.stdout)?;
        println!("{}", output_str);
        Ok(format!("{}.bls.key.json", keyname))
    }
}
