use std::{
    fmt::Debug,
    path::{Path, PathBuf},
    str::FromStr,
};

use async_trait::async_trait;
use ethers::{
    core::{rand::thread_rng, types::Signature, utils::hex::ToHex},
    signers::{LocalWallet, Signer, WalletError},
    types::{
        transaction::{eip2718::TypedTransaction, eip712::Eip712},
        Address,
    },
};
use ivynet_io::{read_json, write_json, IoError};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod bls;
pub mod keychain;
pub mod sign_utils;
pub mod keyfile;

// TODO: Make this a newtype strict and impl deref + derefmut to get signer stuff for free
#[derive(Clone, Debug, PartialEq)]
pub struct IvyWallet {
    local_wallet: LocalWallet,
}

#[derive(thiserror::Error, Debug)]
pub enum IvyWalletError {
    #[error(transparent)]
    WalletError(#[from] WalletError),
    #[error(transparent)]
    IoError(#[from] IoError),
}

impl IvyWallet {
    pub fn new() -> Self {
        let local_wallet = LocalWallet::new(&mut thread_rng());
        IvyWallet { local_wallet }
    }

    pub fn from_private_key(private_key_string: String) -> Result<Self, IvyWalletError> {
        let local_wallet = LocalWallet::from_str(&private_key_string)?;
        Ok(IvyWallet { local_wallet })
    }

    pub fn from_keystore(path: PathBuf, password: &str) -> Result<Self, IvyWalletError> {
        let local_wallet = LocalWallet::decrypt_keystore(path, password)?;
        Ok(IvyWallet { local_wallet })
    }

    pub fn encrypt_and_store(
        &self,
        path: &Path,
        name: String,
        password: String,
    ) -> Result<PathBuf, IvyWalletError> {
        _ = LocalWallet::encrypt_keystore(
            path,
            &mut thread_rng(),
            self.local_wallet.signer().to_bytes(),
            &password,
            Some(&name),
        )?;

        let prv_key_path = path.join(name);

        let Keyfile { crypto, id, version } = read_json(&prv_key_path)?;
        let keyfile = KeyfileLegacy { address: self.local_wallet.address(), crypto, id, version };

        write_json(&prv_key_path, &keyfile)?;

        Ok(prv_key_path)
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

    async fn sign_message<S: Send + Sync + AsRef<[u8]>>(
        &self,
        message: S,
    ) -> Result<Signature, Self::Error> {
        self.local_wallet.sign_message(message).await
    }

    async fn sign_typed_data<T: Eip712 + Send + Sync>(
        &self,
        payload: &T,
    ) -> Result<Signature, Self::Error> {
        self.local_wallet.sign_typed_data(payload).await
    }

    async fn sign_transaction(&self, message: &TypedTransaction) -> Result<Signature, Self::Error> {
        self.local_wallet.sign_transaction(message).await
    }
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

    #[test]
    fn test_wallet_from_private_key() {
        let wallet = IvyWallet::new();
        let private_key = wallet.to_private_key();
        let wallet2 = IvyWallet::from_private_key(private_key).unwrap();
        assert_eq!(wallet.address(), wallet2.address());
    }

    #[test]
    fn test_wallet_from_keystore() {
        let dir = tempdir().unwrap();
        let wallet = IvyWallet::new();
        let address = wallet.address();
        let prv_key_path = wallet
            .encrypt_and_store(dir.as_ref(), "temp_key".to_string(), "ThisIsATempKey".to_string())
            .unwrap();
        let wallet2 = IvyWallet::from_keystore(prv_key_path, "ThisIsATempKey").unwrap();
        assert_eq!(address, wallet2.address());
    }
}
