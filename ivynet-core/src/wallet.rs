use std::{fs, path::PathBuf};

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
use tracing::info;

use crate::error::IvyError;

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

    pub fn encrypt_and_store(&self, name: String, password: String) -> Result<(PathBuf, PathBuf), IvyError> {
        let mut file_path = dirs::home_dir().ok_or(IvyError::DirInaccessible)?;
        file_path.push(".ivynet");

        fs::create_dir_all(&file_path)?;

        LocalWallet::encrypt_keystore(
            file_path.clone(),
            &mut thread_rng(),
            self.local_wallet.signer().to_bytes(),
            password,
            Some(&(name.clone() + ".json")),
        )?;

        let prv_key_path = file_path.join(format!("{name}.json"));
        let pub_key_path = file_path.join(format!("{name}.txt"));

        fs::write(pub_key_path.clone(), self.local_wallet.address())?;
        info!("keyfile stored to {}", file_path.display());

        Ok((pub_key_path, prv_key_path))
    }

    pub fn to_private_key(&self) -> String {
        self.local_wallet.signer().to_bytes().encode_hex::<String>()
    }

    pub fn address(&self) -> Address {
        self.local_wallet.address()
    }

    pub fn address_from_file(path: PathBuf) -> Result<H160, IvyError> {
        let addr_vec: Vec<u8> = fs::read(path)?;
        Ok(H160::from_slice(&addr_vec))
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
