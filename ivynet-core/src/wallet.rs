use std::{
    fmt::Debug,
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use async_trait::async_trait;
use ethers::{
    core::{
        rand::thread_rng,
        types::Signature,
        utils::hex::{FromHex, ToHex},
    },
    signers::{LocalWallet, Signer, WalletError},
    types::{
        transaction::{eip2718::TypedTransaction, eip712::Eip712},
        Address, H160,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{debug, info};

use crate::{
    error::IvyError,
    utils::{read_json, write_json},
};

#[derive(Clone, Debug)]
pub struct IvyWallet {
    local_wallet: LocalWallet,
}

impl IvyWallet {
    pub fn new() -> Self {
        let local_wallet = LocalWallet::new(&mut thread_rng());
        IvyWallet { local_wallet }
    }

    pub fn from_private_key(private_key_string: String) -> Result<Self, IvyError> {
        let priv_bytes = <Vec<u8>>::from_hex(private_key_string)?;
        let local_wallet = LocalWallet::from_bytes(&priv_bytes)?;

        Ok(IvyWallet { local_wallet })
    }

    pub fn from_keystore(path: PathBuf, password: String) -> Result<Self, IvyError> {
        let local_wallet = LocalWallet::decrypt_keystore(path, password)?;

        Ok(IvyWallet { local_wallet })
    }

    pub fn encrypt_and_store(
        &self,
        path: &Path,
        name: String,
        password: String,
    ) -> Result<(PathBuf, PathBuf), IvyError> {
        let encrypt = LocalWallet::encrypt_keystore(
            path,
            &mut thread_rng(),
            self.local_wallet.signer().to_bytes(),
            &password,
            Some(&(name.clone() + ".json")),
        )?;
        debug!("{:?}", encrypt);

        let pub_key_path = path.join(format!("{name}.txt"));
        let prv_key_path = path.join(format!("{name}.json"));
        let address_write = format!("{:?}", self.local_wallet.address());

        fs::write(pub_key_path.clone(), address_write)?;
        info!("keyfile stored to {}", path.display());
        create_legacy_keyfile(&prv_key_path, &password)?;

        Ok((pub_key_path, prv_key_path))
    }

    pub fn to_private_key(&self) -> String {
        self.local_wallet.signer().to_bytes().encode_hex::<String>()
    }

    pub fn signer(&self) -> LocalWallet {
        self.local_wallet.clone()
    }

    pub fn address(&self) -> Address {
        self.local_wallet.address()
    }

    pub fn address_from_file(path: PathBuf) -> Result<H160, IvyError> {
        let addr: String = fs::read_to_string(path)?;
        let addr: H160 = H160::from_str(&addr).map_err(|_| IvyError::InvalidAddress)?;
        Ok(addr)
    }
}

impl Default for IvyWallet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Signer for IvyWallet {
    type Error = WalletError;

    fn address(&self) -> Address {
        self.local_wallet.address()
    }

    fn chain_id(&self) -> u64 {
        self.local_wallet.chain_id()
    }

    fn with_chain_id<T: Into<u64>>(self, chain_id: T) -> Self {
        let local_wallet = self.local_wallet.with_chain_id(chain_id);
        IvyWallet { local_wallet }
    }

    async fn sign_message<S: Send + Sync + AsRef<[u8]>>(&self, message: S) -> Result<Signature, Self::Error> {
        self.local_wallet.sign_message(message).await
    }

    async fn sign_typed_data<T: Eip712 + Send + Sync>(&self, payload: &T) -> Result<Signature, Self::Error> {
        self.local_wallet.sign_typed_data(payload).await
    }

    async fn sign_transaction(&self, message: &TypedTransaction) -> Result<Signature, Self::Error> {
        self.local_wallet.sign_transaction(message).await
    }
}

pub fn create_legacy_keyfile(path: &PathBuf, password: &str) -> Result<(), IvyError> {
    debug!("creating legacy keyfile");
    let wallet = IvyWallet::from_keystore(path.to_owned(), password.to_owned())?;
    debug!("wallet loaded");
    let Keyfile { crypto, id, version } = read_json(path.clone())?;
    let legacy_keyfile = KeyfileLegacy { address: wallet.address(), crypto, id, version };
    let mut legacy_keyfile_path = path.to_owned();
    legacy_keyfile_path.set_extension("legacy.json");
    debug!("{:#?}", legacy_keyfile_path.clone());
    write_json(legacy_keyfile_path, &legacy_keyfile)?;
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct Keyfile {
    crypto: Value,
    id: String,
    version: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct KeyfileLegacy {
    address: Address,
    crypto: Value,
    id: String,
    version: u32,
}

#[cfg(test)]
mod test {
    use super::*;
    use tempfile::tempdir;

    /// Creates a new keyfile and calls address_from_file
    #[test]
    fn test_address_from_file() {
        let dir = tempdir().unwrap();
        let wallet = IvyWallet::new();
        let address = wallet.address();
        wallet.encrypt_and_store(dir.as_ref(), "temp_key".to_string(), "ThisIsATempKey".to_string()).unwrap();
        let addr_path = dir.path().join("temp_key.txt");
        let derived_address = IvyWallet::address_from_file(addr_path).unwrap();
        assert_eq!(address, derived_address);
    }
}
