use std::{fs, path::PathBuf};

use crate::{
    bls::{decode_address, encode_address, Address as BlsAddress, BlsKey},
    error::IvyError,
    wallet::IvyWallet,
};

use env_home::env_home_dir as home_dir;
use ethers::types::Address;

pub enum KeyType {
    Ecdsa,
    Bls,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Key {
    Ecdsa(IvyWallet),
    Bls(BlsKey),
}

impl Key {
    pub fn address(&self) -> KeyAddress {
        match &self {
            Key::Ecdsa(wallet) => KeyAddress::Ecdsa(wallet.address()),
            Key::Bls(key) => KeyAddress::Bls(Box::new(key.address())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyAddress {
    Ecdsa(Address),
    Bls(Box<BlsAddress>),
}

pub struct Keychain {
    path: PathBuf,
}

impl Default for Keychain {
    fn default() -> Self {
        Self { path: home_dir().expect("System without home directory.") }
    }
}

impl Keychain {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn list(&self) -> Result<Vec<KeyAddress>, IvyError> {
        let paths = fs::read_dir(&self.path)?;

        let mut list = Vec::new();
        for path in paths.flatten() {
            let filename = path.file_name();
            let cmps = filename.to_str().unwrap().split('.').collect::<Vec<&str>>();
            if cmps.len() == 3 {
                match cmps[1] {
                    "bls" => {
                        if let Ok(address) = decode_address(cmps[0]) {
                            list.push(KeyAddress::Bls(Box::new(address)));
                        }
                    }
                    // PublicKey struct of theirs. Stupid
                    "ecdsa" => {
                        if let Ok(address) = cmps[0].parse::<Address>() {
                            list.push(KeyAddress::Ecdsa(address));
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(list)
    }

    pub fn generate(&self, key_type: KeyType, password: &str) -> Key {
        match key_type {
            KeyType::Ecdsa => Key::Ecdsa(self.ecdsa_generate(password)),
            KeyType::Bls => Key::Bls(self.bls_generate(password)),
        }
    }

    pub fn import(&self, key_type: KeyType, key: &str, password: &str) -> Result<Key, IvyError> {
        match key_type {
            KeyType::Ecdsa => Ok(Key::Ecdsa(self.ecdsa_import(key, password)?)),
            KeyType::Bls => Ok(Key::Bls(self.bls_import(key, password)?)),
        }
    }

    pub fn load(&self, address: KeyAddress, password: &str) -> Result<Key, IvyError> {
        match address {
            KeyAddress::Ecdsa(address) => Ok(Key::Ecdsa(self.ecdsa_load(address, password)?)),
            KeyAddress::Bls(address) => Ok(Key::Bls(self.bls_load(&address, password)?)),
        }
    }

    fn bls_generate(&self, password: &str) -> BlsKey {
        let bls = BlsKey::new();
        if let Ok(address) = encode_address(&bls.address()) {
            _ = bls.encrypt_and_store(
                &self.path,
                format!("{}.bls.json", address),
                password.to_string(),
            );
        }
        bls
    }

    fn bls_import(&self, key: &str, password: &str) -> Result<BlsKey, IvyError> {
        let bls = BlsKey::from_private_key(key.to_string())?;
        _ = bls.encrypt_and_store(
            &self.path,
            format!("{}.bls.json", encode_address(&bls.address())?),
            password.to_string(),
        );
        Ok(bls)
    }

    fn bls_load(&self, address: &BlsAddress, password: &str) -> Result<BlsKey, IvyError> {
        Ok(BlsKey::from_keystore(
            self.path.join(format!("{}.bls.json", encode_address(address)?)),
            password,
        )?)
    }

    fn ecdsa_generate(&self, password: &str) -> IvyWallet {
        let wallet = IvyWallet::new();
        _ = wallet.encrypt_and_store(
            &self.path,
            format!("{:?}.ecdsa", wallet.address()),
            password.to_string(),
        );
        wallet
    }

    fn ecdsa_import(&self, key: &str, password: &str) -> Result<IvyWallet, IvyError> {
        let wallet = IvyWallet::from_private_key(key.to_string())?;
        _ = wallet.encrypt_and_store(
            &self.path,
            format!("{:?}.ecdsa", wallet.address()),
            password.to_string(),
        );
        Ok(wallet)
    }

    fn ecdsa_load(&self, address: Address, password: &str) -> Result<IvyWallet, IvyError> {
        IvyWallet::from_keystore(self.path.join(format!("{:?}.ecdsa.json", address)), password)
    }
}

#[cfg(test)]
pub mod test {
    use std::future::Future;

    use super::*;
    use tokio::fs;

    pub async fn build_test_dir<F, Fut, T>(test_dir: &str, test_logic: F) -> T
    where
        F: FnOnce(PathBuf) -> Fut,
        Fut: Future<Output = T>,
    {
        let test_path = std::env::temp_dir().join(format!("testing_{}", test_dir));
        // Folder might have existed before
        _ = fs::remove_dir_all(test_path.clone()).await;
        fs::create_dir_all(&test_path).await.expect("Failed to create testing_temp directory");
        let result = test_logic(test_path.clone()).await;
        fs::remove_dir_all(test_path).await.expect("Failed to delete testing_temp directory");

        result
    }

    #[tokio::test]
    async fn test_ecdsa_key_generation_and_load() {
        build_test_dir("keychain_ecdsa", |test_path| async move {
            let keychain = Keychain::new(test_path);

            let ecdsa = keychain.generate(KeyType::Ecdsa, "testpws");
            let all_keys = keychain.list().unwrap();
            assert_eq!(all_keys.len(), 1);
            assert_eq!(ecdsa.address(), all_keys[0]);

            let loaded_ecdsa = keychain.load(all_keys[0].clone(), "testpws").unwrap();
            assert_eq!(ecdsa, loaded_ecdsa);
        })
        .await;
    }

    #[tokio::test]
    async fn test_bls_key_generation_and_load() {
        build_test_dir("keychain_bls", |test_path| async move {
            let keychain = Keychain::new(test_path);

            let bls = keychain.generate(KeyType::Bls, "testpws");
            let all_keys = keychain.list().unwrap();
            assert_eq!(all_keys.len(), 1);
            assert_eq!(bls.address(), all_keys[0]);

            let loaded_bls = keychain.load(all_keys[0].clone(), "testpws").unwrap();
            assert_eq!(bls, loaded_bls);
        })
        .await;
    }

    #[tokio::test]
    async fn test_multiple_keys_gen() {
        build_test_dir("keychain_multi", |test_path| async move {
            let keychain = Keychain::new(test_path);

            let bls = keychain.generate(KeyType::Bls, "testpws");
            let ecdsa = keychain.generate(KeyType::Ecdsa, "testpws");
            let all_keys = keychain.list().unwrap();
            assert_eq!(all_keys.len(), 2);
            for key in all_keys {
                match key {
                    KeyAddress::Bls(_) => assert_eq!(bls.address(), key),
                    KeyAddress::Ecdsa(_) => assert_eq!(ecdsa.address(), key),
                }
            }
        })
        .await;
    }
}
