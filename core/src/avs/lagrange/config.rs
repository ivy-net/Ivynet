use dialoguer::Input;
use ethers::types::Address;
use ivynet_docker::dockercmd::DockerCmd;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
};
use tracing::{debug, error, info};
use url::Url;

use crate::{
    avs::config::{default_config_dir, NodeConfigError},
    env_parser::EnvLines,
    io::{read_toml, unzip_to, IoError},
    keychain::{KeyType, Keychain},
};

const LAGRANGE_WORKER_SETUP_REPO: &str =
    "https://github.com/ivy-net/lagrange-worker/archive/refs/heads/main.zip";

/// Config type for the lagrange worker, defined in worker-conf.toml of the Lagrange spec.
/// stored locally in ${LAGRANGE_WORKER_DIR}/worker-conf.toml
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LagrangeConfig {
    pub path: PathBuf,
    pub compose_file: PathBuf,
    pub node_directory: PathBuf,
    pub ecdsa_keyfile: PathBuf,
    pub operator_address: Address,
    pub rpc_url: Url,
    pub worker_id: String,
}

impl LagrangeConfig {
    pub fn load(path: PathBuf) -> Result<Self, IoError> {
        let config: Self = read_toml(&path)?;
        Ok(config)
    }

    pub fn store(&self, path: PathBuf) -> Result<(), IoError> {
        crate::io::write_toml(&path, &self)
    }
}

impl LagrangeConfig {
    pub fn name(&self) -> String {
        self.path
            .file_stem()
            .expect("Could not get file stem")
            .to_str()
            .expect("Could not convert file stem to string")
            .to_string()
    }

    pub async fn new_from_prompt() -> Result<Self, NodeConfigError> {
        let node_name = dialoguer::Input::<String>::new()
        .with_prompt(
            "Enter the name of the node instance. This name will be used for later identification",
        )
        .interact()?;

        let config_path = default_config_dir().join(format!("{}.toml", node_name));
        let node_directory = prompt_lagrange_directory()?;
        download_operator_setup(&node_directory).await?;

        let sample_holesky_compose =
            node_directory.clone().join("lagrange-worker-main/holesky/docker-compose.yaml");
        let sample_mainnet_compose =
            node_directory.clone().join("lagrange-worker-main/mainnet/docker-compose.yaml");

        // TODO: This is a bit verbose. Consider including an example config file in
        // deployments instead.
        let compose_file: PathBuf = dialoguer::Input::<String>::new()
            .with_prompt(format!("Enter the path to the docker-compose file. For Lagrange-worker nodes, this will be usually be located at \n{:?} \nfor standard Holesky deployments or \n{:?} \nfor standard Mainnet deployments.", sample_holesky_compose, sample_mainnet_compose))
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

        let rpc_url = dialoguer::Input::<String>::new()
            .with_prompt("Enter the RPC URL")
            .interact_text()?
            .parse::<Url>()?;

        let worker_id = dialoguer::Input::<String>::new()
            .with_prompt("Enter a worker ID for the node")
            .interact_text()?;

        let config = LagrangeConfig {
            path: config_path,
            compose_file,
            node_directory,
            ecdsa_keyfile,
            operator_address,
            rpc_url,
            worker_id,
        };

        let gen_lagr_key = dialoguer::Confirm::new()
            .with_prompt(
                "Would you like to generate a new Lagrange key? This will overwrite your old key, ensure that you have backed up old keys for recovery.",
            )
            .interact()?;

        let lagr_keyfile_pw = dialoguer::Password::new()
            .with_prompt("Enter the password for the Lagrange keyfile. This is required by the lagrange node, and is stored in plaintext in the lagrange .env file per their specs.").with_confirmation("Confirm password", "Passwords do not match")
            .interact()?;

        if gen_lagr_key {
            generate_lagrange_key(&config.compose_file).await?;
        }

        build_env(&config, &lagr_keyfile_pw).await?;
        Ok(config)
    }
}

pub async fn download_operator_setup(dest_dir: &Path) -> Result<(), NodeConfigError> {
    // Resource directory setup
    let mut setup = false;
    if dest_dir.exists() {
        let reset_string: String = Input::new()
            .with_prompt(
                "The setup directory already exists. Clear directory and redownload? (y/n)",
            )
            .interact_text()?;

        if reset_string == "y" {
            setup = true;
            fs::remove_dir_all(dest_dir)?;
        }
    } else {
        setup = true;
    }

    if setup {
        info!("Downloading setup files to {}", dest_dir.display());
        fs::create_dir_all(dest_dir)?;
        let response = reqwest::get(LAGRANGE_WORKER_SETUP_REPO).await?;
        let zip_fname = dest_dir.join("source.zip");
        let mut zip_dest = File::create(&zip_fname)?;
        let bytes = response.bytes().await?;
        std::io::copy(&mut bytes.as_ref(), &mut zip_dest)?;

        unzip_to(&zip_fname, dest_dir)?;
        std::fs::remove_file(zip_fname)?;

        // let reader = BufReader::new(File::open(fname)?);
        // let mut archive = ZipArchive::new(reader)?;

        // for i in 0..archive.len() {
        //     let mut file = archive.by_index(i)?;
        //     let outpath = temp_path.join(file.name());
        //     debug!("Extracting to {}", outpath.display());

        //     if (file.name()).ends_with('/') {
        //         std::fs::create_dir_all(&outpath)?;
        //     } else {
        //         if let Some(p) = outpath.parent() {
        //             if !p.exists() {
        //                 std::fs::create_dir_all(p)?;
        //             }
        //         }
        //         let mut outfile = File::create(&outpath)?;
        //         std::io::copy(&mut file, &mut outfile)?;
        //     }
        // }
        // let first_dir = std::fs::read_dir(&temp_path)?
        //     .filter_map(Result::ok)
        //     .find(|entry| entry.file_type().unwrap().is_dir());
        // println!("first dir: {:?}", first_dir);
        // println!("dest dir: {:?}", dest_dir);
        // if let Some(first_dir) = first_dir {
        //     let old_folder_path = first_dir.path();
        //     debug!("{}", old_folder_path.display());
        //     std::fs::rename(old_folder_path, dest_dir)?;
        // }
        // println!("rename complete");
        // // Delete the setup directory
        // if temp_path.exists() {
        //     info!("Cleaning up setup directory...");
        //     std::fs::remove_dir_all(temp_path)?;
        // }
        // println!("cleanup complete");
    }

    Ok(())
}

fn prompt_lagrange_directory() -> Result<PathBuf, NodeConfigError> {
    let opt = ["Use default", "Enter custom path"];
    let selection = dialoguer::Select::new()
        .items(&opt)
        .default(0)
        .with_prompt("Would you like to use the default Lagrange Worker resources directory or enter a custom path for an already existing directory? If no Lagrange Worker resource directory exists at the chosen path, the resource package will be downloaded to that location.")
        .interact()?;
    match selection {
        0 => Ok(default_lagrange_worker_resources_dir()),
        1 => {
            let path = dialoguer::Input::<String>::new()
                .with_prompt("Enter the path to the node resources directory")
                .interact()?;
            Ok(path.into())
        }
        _ => panic!("Invalid selection"),
    }
}

async fn build_env(config: &LagrangeConfig, lagr_keyfile_pw: &str) -> Result<(), NodeConfigError> {
    let avs_run_path = config
        .compose_file
        .clone()
        .parent()
        .expect("Could not get parent directory of compose filepath")
        .to_path_buf();
    let env_example_path = avs_run_path.join(".env.example");
    let env_path = avs_run_path.join(".env");

    if !env_example_path.exists() {
        error!("The '.env.example' file does not exist at {}. '.env.example' is used for .env templating, please ensure the operator-setup was downloaded to the correct location.", env_example_path.display());
        return Err(NodeConfigError::NoEnvExample);
    }
    std::fs::copy(env_example_path, env_path.clone())?;

    debug!("configuring env...");
    debug!("{}", env_path.display());
    let mut env_lines = EnvLines::load(&env_path)?;
    env_lines.set("AVS__LAGR_PWD", lagr_keyfile_pw);
    env_lines.set("LAGRANGE_RPC_URL", config.rpc_url.as_ref());

    // Setting the "NETWORK" field is preserved here, as the default Lagrange setup requires
    // that you edit the docker-compose.yml directly to change the network based on chain name
    // in this field, whereas the ivynet fork allows direct RPC setting.
    // let chain = {
    //     let provider = Provider::try_from(config.rpc_url.as_ref())?;
    //     let chain_id = provider.get_chainid().await?;
    //     Chain::from(chain_id)
    // }
    // env_lines.set("NETWORK", self.chain.as_ref());

    env_lines.save(&env_path)?;
    Ok(())
}

pub fn default_lagrange_worker_resources_dir() -> PathBuf {
    dirs::home_dir().unwrap().join(".eigenlayer/lagrange_worker")
}

pub async fn generate_lagrange_key(path: &Path) -> Result<(), NodeConfigError> {
    let _ = DockerCmd::new()
        .await?
        .args(["compose", "run", "--rm", "worker", "avs", "new-key"])
        .current_dir(path)
        .status()
        .await?;

    Ok(())
}
