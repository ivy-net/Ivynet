use std::{
    fs::{self, File},
    io::{copy, BufReader},
    path::{Path, PathBuf},
};

use dialoguer::Input;
use ethers::types::Address;
use serde::{Deserialize, Serialize};
use tokio::process::Child;
use tracing::{debug, error, info};
use url::Url;
use zip::ZipArchive;

use crate::{
    avs::config::{default_config_dir, NodeConfig, NodeConfigError},
    docker::dockercmd::DockerCmd,
    download::dl_progress_bar,
    env_parser::EnvLines,
    error::IvyError,
    keychain::{KeyType, Keychain},
};

pub const EIGENDA_SETUP_REPO: &str =
    "https://github.com/ivy-net/eigenda-operator-setup/archive/refs/heads/master.zip";

/// EigenDA node configuration. Mostly a reflection of the AvsConfig struct, with the node_data
/// field pulled out of the NodeConfigData enum for easier access.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EigenDAConfig {
    pub path: PathBuf,
    pub compose_file: PathBuf,
    /// Directory containing the EigenDA node resources
    pub node_directory: PathBuf,
    /// Ecdsa keyfile for the operator
    pub ecdsa_keyfile: PathBuf,
    /// Bls keyfile for the operator
    pub bls_keyfile: PathBuf,
    /// Decrypted operator address,
    pub operator_address: Address,
    /// RPC URL for node connectivity to chain
    pub rpc_url: Url,
}

impl TryFrom<NodeConfig> for EigenDAConfig {
    type Error = IvyError;

    fn try_from(node_config: NodeConfig) -> Result<Self, Self::Error> {
        match node_config {
            NodeConfig::EigenDA(eigenda_config) => Ok(eigenda_config),
            _ => Err(IvyError::ConfigMatchError(
                "EigenDA".to_string(),
                node_config.node_type().to_string(),
            )),
        }
    }
}

impl EigenDAConfig {
    /// Start the EigenDA node
    pub async fn start(&self) -> Result<Child, IvyError> {
        let compose_filename = self
            .compose_file
            .file_name()
            .ok_or_else(|| {
                IvyError::InvalidDockerCompose("Compose file path is invalid".to_string())
            })?
            .to_str()
            .ok_or_else(|| {
                IvyError::InvalidDockerCompose("Compose file path is invalid".to_string())
            })?;

        let parent_dir = self.compose_file.parent().ok_or_else(|| {
            IvyError::InvalidDockerCompose("Compose file path is invalid".to_string())
        })?;

        Ok(DockerCmd::new()
            .await?
            .current_dir(parent_dir)
            .args(["-f", compose_filename, "up", "--force-recreate", "-d"])
            .spawn()?)
    }

    /// Filename of the config file
    pub fn name(&self) -> String {
        let name = self
            .path
            .file_stem()
            .expect("Could not extract filename from path.")
            .to_str()
            .expect("String conversion error for filename")
            .to_string();
        name
    }

    /// Prompt the user for configuration details and create a new EigenDAConfig
    pub async fn new_from_prompt() -> Result<Self, NodeConfigError> {
        // Resource directory setup
        let node_name =
            dialoguer::Input::<String>::new().with_prompt("Enter the name of the node instance. This name will be used for later identification").interact()?;

        let config_path = default_config_dir().join(format!("{}.toml", node_name));

        let node_directory = prompt_eigenda_directory()?;
        download_operator_setup(&node_directory).await?;
        download_g1_g2(&node_directory).await?;

        let sample_holesky_compose =
            node_directory.clone().join("eigenda-operator-setup/holesky/docker-compose.yml");
        let sample_mainnet_compose =
            node_directory.clone().join("eigenda-operator-setup/mainnet/docker-compose.yml");

        // TODO: This is a bit verbose. Consider including an example config file in
        // deployments instead.
        let compose_file: PathBuf = dialoguer::Input::<String>::new()
            .with_prompt(format!("Enter the path to the docker-compose file. For EigenDA nodes, this will be usually be located at \n{:?} \nfor standard Holesky deployments or \n{:?} \nfor standard Mainnet deployments.", sample_holesky_compose, sample_mainnet_compose))
            .interact()?
            .into();

        if !compose_file.exists() {
            // TODO: Anyhow error msg
            return Err(NodeConfigError::FileNotFound(
                compose_file.to_str().expect("String conversion failed for pathbuf").to_string(),
            ));
        }

        let keychain = Keychain::default();
        let ecdsa_keyname = keychain.select_key(KeyType::Ecdsa)?;
        let ecdsa_keyfile = keychain.get_path(&ecdsa_keyname);
        let operator_address = keychain.public_address(&ecdsa_keyname)?.parse()?;

        let bls_keyname = keychain.select_key(KeyType::Bls)?;
        let bls_keyfile = keychain.get_path(&bls_keyname);

        let rpc_url = dialoguer::Input::<String>::new()
            .with_prompt("Enter the RPC URL")
            .interact_text()?
            .parse::<Url>()?;

        let config = Self {
            path: config_path,
            node_directory,
            compose_file,
            ecdsa_keyfile,
            bls_keyfile,
            operator_address,
            rpc_url,
        };

        let bls_key_password = dialoguer::Password::new()
            .with_prompt("Enter the password for the configured BLS keyfile")
            .interact()?;

        build_env(&config, &bls_key_password).await?;

        println!("New EigenDA node configuration successfully created at {:?}", config.path);

        Ok(config)
    }
}

impl Default for EigenDAConfig {
    fn default() -> Self {
        let path = default_config_dir();
        let compose_file = "".into();
        let node_directory = "".into();
        let ecdsa_keyfile = "".into();
        let bls_keyfile = "".into();
        let operator_address = Address::zero();
        let rpc_url = Url::parse("https://rpc.flashbots.net/fast").unwrap();

        Self {
            path,
            compose_file,
            node_directory,
            ecdsa_keyfile,
            bls_keyfile,
            operator_address,
            rpc_url,
        }
    }
}

fn prompt_eigenda_directory() -> Result<PathBuf, NodeConfigError> {
    let opt = ["Use default", "Enter custom path"];
    let selection = dialoguer::Select::new()
        .items(&opt)
        .default(0)
        .with_prompt("Would you like to use the default EigenDA resources directory or enter a custom path for an already existing directory? If no EigenDA resource directory exists at the chosen path, the resource package will be downloaded to that location.")
        .interact()?;
    match selection {
        0 => Ok(default_eigenda_resources_dir()),
        1 => {
            let path = dialoguer::Input::<String>::new()
                .with_prompt("Enter the path to the node resources directory")
                .interact()?;
            Ok(path.into())
        }
        _ => panic!("Invalid selection"),
    }
}

pub fn default_eigenda_resources_dir() -> PathBuf {
    dirs::home_dir().unwrap().join(".eigenlayer/eigenda")
}

pub async fn download_operator_setup(eigen_path: &Path) -> Result<(), NodeConfigError> {
    let mut setup = false;
    let temp_path = eigen_path.join("temp");
    let destination_path = eigen_path.join("eigenda-operator-setup");
    if destination_path.exists() {
        let reset_string: String = Input::new()
            .with_prompt("The operator setup directory already exists. Clear directory and redownload? (y/n)")
            .interact_text()?;

        if reset_string == "y" {
            setup = true;
            fs::remove_dir_all(destination_path.clone())?;
            fs::create_dir_all(temp_path.clone())?;
        }
    } else {
        info!("The setup directory does not exist, downloading to {}", temp_path.display());
        fs::create_dir_all(temp_path.clone())?;
        setup = true;
    }

    if setup {
        info!("Downloading setup files to {}", temp_path.display());
        let response = reqwest::get(EIGENDA_SETUP_REPO).await?;

        let fname = temp_path.join("source.zip");
        let mut dest = File::create(&fname)?;
        let bytes = response.bytes().await?;
        std::io::copy(&mut bytes.as_ref(), &mut dest)?;
        let reader = BufReader::new(File::open(fname)?);
        let mut archive = ZipArchive::new(reader)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = temp_path.join(file.name());
            debug!("Extracting to {}", outpath.display());

            if (file.name()).ends_with('/') {
                std::fs::create_dir_all(&outpath)?;
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        std::fs::create_dir_all(p)?;
                    }
                }
                let mut outfile = File::create(&outpath)?;
                copy(&mut file, &mut outfile)?;
            }
        }
        let first_dir = std::fs::read_dir(&temp_path)?
            .filter_map(Result::ok)
            .find(|entry| entry.file_type().unwrap().is_dir());
        if let Some(first_dir) = first_dir {
            let old_folder_path = first_dir.path();
            debug!("{}", old_folder_path.display());
            std::fs::rename(old_folder_path, destination_path)?;
        }
        // Delete the setup directory
        if temp_path.exists() {
            info!("Cleaning up setup directory...");
            std::fs::remove_dir_all(temp_path)?;
        }
    }

    Ok(())
}

async fn build_env(config: &EigenDAConfig, bls_key_password: &str) -> Result<(), NodeConfigError> {
    let avs_run_path = config
        .compose_file
        .clone()
        .parent()
        .expect("Could not get parent directory of compose filepath")
        .to_path_buf();
    let rpc_url = config.rpc_url.clone();

    let env_example_path = avs_run_path.join(".env.example");
    let env_path = avs_run_path.join(".env");

    if !env_example_path.exists() {
        error!("The '.env.example' file does not exist at {}. '.env.example' is used for .env templating, please ensure the operator-setup was downloaded to the correct location.", env_example_path.display());
        return Err(NodeConfigError::NoEnvExample);
    }
    std::fs::copy(env_example_path, env_path.clone())?;

    debug!("configuring env...");
    let mut env_lines = EnvLines::load(&env_path)?;

    let node_hostname = reqwest::get("https://api.ipify.org").await?.text().await?;
    info!("Using node hostname: {node_hostname}");

    env_lines.set("NODE_HOSTNAME", &node_hostname);
    env_lines.set("NODE_CHAIN_RPC", rpc_url.as_ref());

    // User home directory
    let home_dir = dirs::home_dir().expect("Could not get home directory");
    let home_str = home_dir.to_str().expect("Could not get home directory");
    env_lines.set("USER_HOME", home_str);

    // Node resource paths
    env_lines.set("NODE_G1_PATH_HOST", r#"${EIGENLAYER_HOME}/eigenda/resources/g1.point"#);
    env_lines.set("NODE_G2_PATH_HOST", r#"${EIGENLAYER_HOME}/eigenda/resources/g2.point.powerOf2"#);

    env_lines.set(
        "NODE_CACHE_PATH_HOST",
        r#"${EIGENLAYER_HOME}/eigenda/eigenda-operator-setup/resources/cache"#,
    );
    env_lines.set(
        "NODE_BLS_KEY_FILE_HOST",
        config.bls_keyfile.to_str().expect("Could not get BLS key file location"),
    );
    env_lines.set("NODE_BLS_KEY_PASSWORD", &format!("'{}'", bls_key_password));

    env_lines.save(&env_path)?;

    info!(".env file saved to {}", env_path.display());

    Ok(())
}

pub async fn download_g1_g2(eigen_path: &Path) -> Result<(), NodeConfigError> {
    let resources_dir = eigen_path.join("resources");
    fs::create_dir_all(resources_dir.clone())?;
    let g1_file_path = resources_dir.join("g1.point");
    let g2_file_path = resources_dir.join("g2.point.powerOf2");
    if g1_file_path.exists() {
        info!("The 'g1.point' file already exists.");
    } else {
        info!("Downloading 'g1.point'  to {}", g1_file_path.display());
        dl_progress_bar("https://srs-mainnet.s3.amazonaws.com/kzg/g1.point", g1_file_path).await?;
    }
    if g2_file_path.exists() {
        info!("The 'g2.point.powerOf2' file already exists.");
    } else {
        info!("Downloading 'g2.point.powerOf2' ...");
        dl_progress_bar("https://srs-mainnet.s3.amazonaws.com/kzg/g2.point.powerOf2", g2_file_path)
            .await?
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eigenda_config_new_default() {
        let config = EigenDAConfig::default();
        assert_eq!(config.compose_file, PathBuf::from(""));
        assert_eq!(config.node_directory, PathBuf::from(""));
        assert_eq!(config.ecdsa_keyfile, PathBuf::from(""));
        assert_eq!(config.operator_address, Address::zero());
        assert_eq!(config.rpc_url, Url::parse("https://rpc.flashbots.net/fast").unwrap());
        println!("{:?}", config);
    }
}
