use ethers::types::Chain;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tracing::{debug, warn};

use crate::{config::IvyConfig, error::IvyError, ethers::types::Address, wallet::IvyWallet};

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

pub fn parse_chain(chain: &str) -> Chain {
    chain.parse::<Chain>().unwrap_or_else(|_| {
        warn!("unknown network: {chain}, defaulting to anvil_hardhat at 31337");
        Chain::AnvilHardhat
    })
}

pub fn unwrap_or_local(opt_address: Option<Address>, config: IvyConfig) -> Result<Address, IvyError> {
    match opt_address {
        Some(address) => Ok(address),
        None => {
            warn!("No address provided, defaulting to local wallet address");
            IvyWallet::address_from_file(config.default_public_keyfile)
        }
    }
}
