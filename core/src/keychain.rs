use crate::{
    bls::{encode_address, Address as BlsAddress, BlsKey},
    wallet::IvyWallet,
};
use dialoguer::Select;
use serde_json::Value;
use std::{fmt::Display, fs, path::PathBuf};

use ethers::{types::Address, utils::hex::encode};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyType {
    Ecdsa,
    Bls,
}

#[derive(Debug, PartialEq, Eq)]
pub enum KeyAddress {
    Ecdsa(Address),
    Bls(Box<BlsAddress>),
}

impl Display for KeyAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyAddress::Ecdsa(a) => f.write_fmt(format_args!("{a:?}")),
            KeyAddress::Bls(a) => {
                f.write_fmt(format_args!("{}", encode_address(a).expect("Bad address")))
            }
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum Key {
    Ecdsa(IvyWallet),
    Bls(BlsKey),
}

impl Key {
    pub fn get_wallet_owned(&self) -> Option<IvyWallet> {
        if let Key::Ecdsa(wallet) = self {
            Some(wallet.clone())
        } else {
            None
        }
    }

    pub fn get_bls_key_owned(&self) -> Option<BlsKey> {
        if let Key::Bls(bls_key) = self {
            Some(bls_key.clone())
        } else {
            None
        }
    }

    pub fn address(&self) -> KeyAddress {
        match &self {
            Key::Ecdsa(wallet) => KeyAddress::Ecdsa(wallet.address()),
            Key::Bls(key) => KeyAddress::Bls(Box::new(key.address())),
        }
    }

    pub fn ecdsa_address(&self) -> Option<Address> {
        match &self {
            Key::Ecdsa(wallet) => Some(wallet.address()),
            _ => None,
        }
    }

    pub fn bls_address(&self) -> Option<BlsAddress> {
        match &self {
            Key::Bls(key) => Some(key.address()),
            _ => None,
        }
    }

    pub fn is_type(&self, key_type: KeyType) -> bool {
        match &self {
            Key::Bls(_) => key_type == KeyType::Bls,
            Key::Ecdsa(_) => key_type == KeyType::Ecdsa,
        }
    }

    pub fn private_key_string(&self) -> String {
        match &self {
            Key::Bls(key) => encode(key.secret().to_be_bytes()),
            Key::Ecdsa(wallet) => wallet.to_private_key(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyName {
    Ecdsa(String),
    Bls(String),
}

impl Display for KeyName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyName::Ecdsa(n) => f.write_fmt(format_args!("{n}")),
            KeyName::Bls(n) => f.write_fmt(format_args!("{n}")),
        }
    }
}

pub struct Keychain {
    path: PathBuf,
}

impl Default for Keychain {
    fn default() -> Self {
        Self { path: dirs::home_dir().expect("Could not get a home directory").join(".ivynet") }
    }
}

impl Keychain {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn list(&self) -> Result<Vec<KeyName>, KeychainError> {
        let paths = fs::read_dir(&self.path)?;

        let mut list = Vec::new();
        for path in paths.flatten() {
            let filename = path.file_name();
            let cmps = filename.to_str().unwrap().split('.').collect::<Vec<&str>>();
            if cmps.len() == 3 || cmps.len() == 4 {
                match cmps[1] {
                    "bls" => list.push(KeyName::Bls(cmps[0].to_string())),
                    "ecdsa" => list.push(KeyName::Ecdsa(cmps[0].to_string())),
                    _ => {}
                }
            }
        }
        Ok(list)
    }

    pub fn keynames_for_display(&self, key_type: &KeyType) -> Result<Vec<String>, KeychainError> {
        let mut key_strings = Vec::new();
        match self.list() {
            Ok(keys) => {
                for keyname in keys {
                    match key_type {
                        KeyType::Ecdsa => {
                            if let KeyName::Ecdsa(name) = keyname {
                                key_strings.push(name)
                            }
                        }
                        KeyType::Bls => {
                            if let KeyName::Bls(name) = keyname {
                                key_strings.push(name)
                            }
                        }
                    }
                }
                Ok(key_strings)
            }
            Err(e) => Err(e),
        }
    }

    pub fn select_key(&self, key_type: KeyType) -> Result<KeyName, KeychainError> {
        let keys = self.keynames_for_display(&key_type)?;

        if keys.is_empty() {
            return Err(KeychainError::NoKeyFoundError);
        }
        let keys_display: &[String] = &keys;

        if keys.len() == 1 {
            let keyname = &keys[0];
            return match key_type {
                KeyType::Ecdsa => Ok(KeyName::Ecdsa(keyname.to_string())),
                KeyType::Bls => Ok(KeyName::Bls(keyname.to_string())),
            };
        }

        let interactive = Select::new()
            .with_prompt(format!(
                "Which {} key would you like to use?",
                if key_type == KeyType::Bls { "BLS" } else { "ECDSA" }
            ))
            .items(keys_display)
            .default(0)
            .interact()?;

        let keyname = &keys_display[interactive];

        match key_type {
            KeyType::Ecdsa => Ok(KeyName::Ecdsa(keyname.to_string())),
            KeyType::Bls => Ok(KeyName::Bls(keyname.to_string())),
        }
    }

    pub fn generate(&self, key_type: KeyType, name: Option<&str>, password: &str) -> Key {
        match key_type {
            KeyType::Ecdsa => Key::Ecdsa(self.ecdsa_generate(name, password)),
            KeyType::Bls => Key::Bls(self.bls_generate(name, password)),
        }
    }

    pub fn import(
        &self,
        key_type: KeyType,
        name: Option<&str>,
        key: &str,
        password: &str,
    ) -> Result<Key, KeychainError> {
        match key_type {
            KeyType::Ecdsa => Ok(Key::Ecdsa(self.ecdsa_import(name, key, password)?)),
            KeyType::Bls => Ok(Key::Bls(self.bls_import(name, key, password)?)),
        }
    }

    pub fn import_from_file(
        &self,
        path: PathBuf,
        key_type: KeyType,
        password: &str,
    ) -> Result<(String, Key), KeychainError> {
        let name = if let Some(n) =
            path.file_name().unwrap().to_str().expect("Unparsable path").split(".").next()
        {
            n.to_string()
        } else {
            path.file_name().unwrap().to_str().unwrap().to_string()
        };
        match key_type {
            KeyType::Ecdsa => {
                let key = IvyWallet::from_keystore(path, password)?;
                _ = key.encrypt_and_store(
                    &self.path,
                    format!("{}.ecdsa.json", &name),
                    password.to_string(),
                );
                Ok((name, Key::Ecdsa(key)))
            }
            KeyType::Bls => {
                let key = BlsKey::from_keystore(path, password)?;
                _ = key.encrypt_and_store(
                    &self.path,
                    format!("{}.bls.json", &name),
                    password.to_string(),
                );
                Ok((name, Key::Bls(key)))
            }
        }
    }

    pub fn load(&self, address: KeyName, password: &str) -> Result<Key, KeychainError> {
        match address {
            KeyName::Ecdsa(name) => Ok(Key::Ecdsa(self.ecdsa_load(&name, password)?)),
            KeyName::Bls(name) => Ok(Key::Bls(self.bls_load(&name, password)?)),
        }
    }

    pub fn get_path(&self, name: &KeyName) -> PathBuf {
        match name {
            KeyName::Ecdsa(name) => self.path.join(format!("{name}.ecdsa.json")),
            KeyName::Bls(name) => self.path.join(format!("{name}.bls.json")),
        }
    }

    pub fn public_address(&self, name: &KeyName) -> Result<String, KeychainError> {
        let path = self.path.join(match &name {
            KeyName::Ecdsa(name) => format!("{name}.ecdsa.json"),
            KeyName::Bls(name) => format!("{name}.bls.json"),
        });
        println!("Reading path {path:?}");
        let json = self.read_json_file(&path)?;
        let address = match json.get(match name {
            KeyName::Ecdsa(_) => "address",
            KeyName::Bls(_) => "pubKey",
        }) {
            Some(value) => value,
            None => return Err(KeychainError::AddressFieldError),
        };
        Ok(address.to_string().trim_matches('"').to_string())
    }

    fn bls_generate(&self, name: Option<&str>, password: &str) -> BlsKey {
        let bls = BlsKey::new();
        _ = bls.encrypt_and_store(
            &self.path,
            Self::gen_keyname(name, "bls", encode_address(&bls.address()).ok()),
            password.to_string(),
        );
        bls
    }

    fn bls_import(
        &self,
        name: Option<&str>,
        key: &str,
        password: &str,
    ) -> Result<BlsKey, KeychainError> {
        let bls = BlsKey::from_private_key(key.to_string())?;
        _ = bls.encrypt_and_store(
            &self.path,
            Self::gen_keyname(name, "bls", encode_address(&bls.address()).ok()),
            password.to_string(),
        );
        Ok(bls)
    }

    fn bls_load(&self, name: &str, password: &str) -> Result<BlsKey, KeychainError> {
        Ok(BlsKey::from_keystore(self.path.join(format!("{name}.bls.json")), password)?)
    }

    fn ecdsa_generate(&self, name: Option<&str>, password: &str) -> IvyWallet {
        let wallet = IvyWallet::new();
        _ = wallet.encrypt_and_store(
            &self.path,
            Self::gen_keyname(name, "ecdsa", Some(format!("{:?}", wallet.address()))),
            password.to_string(),
        );
        wallet
    }

    fn ecdsa_import(
        &self,
        name: Option<&str>,
        key: &str,
        password: &str,
    ) -> Result<IvyWallet, KeychainError> {
        let wallet = IvyWallet::from_private_key(key.to_string())?;
        _ = wallet.encrypt_and_store(
            &self.path,
            Self::gen_keyname(name, "ecdsa", Some(format!("{:?}", wallet.address()))),
            password.to_string(),
        );
        Ok(wallet)
    }

    fn ecdsa_load(&self, name: &str, password: &str) -> Result<IvyWallet, KeychainError> {
        Ok(IvyWallet::from_keystore(self.path.join(format!("{name}.ecdsa.json")), password)?)
    }

    fn gen_keyname(name: Option<&str>, key_type: &str, address_string: Option<String>) -> String {
        match name {
            Some(ref n) => format!("{n}.{key_type}.json"),
            None => match address_string {
                Some(n) => format!("{n}.{key_type}.json"),
                _ => format!("key.{key_type}.json"),
            },
        }
    }

    fn read_json_file(&self, path: &PathBuf) -> Result<Value, KeychainError> {
        let data = fs::read_to_string(path).expect("No data in json");
        let json: Value = serde_json::from_str(&data).expect("Could not parse through json");
        Ok(json)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum KeychainError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    IvyWalletError(#[from] crate::wallet::IvyWalletError),
    #[error(transparent)]
    BlsKeyError(#[from] crate::bls::BlsKeyError),
    #[error(transparent)]
    DialoguerError(#[from] dialoguer::Error),
    #[error("No address field found in keyfile")]
    AddressFieldError,
    // TODO: Test this message
    #[error("No valid key was found. Please create a key with `ivynet key` before trying again.")]
    NoKeyFoundError,
}

#[cfg(test)]
pub mod test {
    use std::future::Future;

    use super::*;
    use ethers::utils::hex::encode;
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

            let ecdsa = keychain.generate(KeyType::Ecdsa, None, "testpws");
            let all_keys = keychain.list().unwrap();
            assert_eq!(all_keys.len(), 1);
            assert_eq!(format!("{:?}", ecdsa.ecdsa_address().unwrap()), all_keys[0].to_string());

            let loaded_ecdsa = keychain.load(all_keys[0].clone(), "testpws").unwrap();
            assert_eq!(ecdsa, loaded_ecdsa);
        })
        .await;
    }

    #[tokio::test]
    async fn test_bls_key_generation_and_load() {
        build_test_dir("keychain_bls", |test_path| async move {
            let keychain = Keychain::new(test_path);

            let bls = keychain.generate(KeyType::Bls, None, "testpws");
            let all_keys = keychain.list().unwrap();
            assert_eq!(all_keys.len(), 1);
            assert_eq!(
                encode_address(&bls.bls_address().unwrap()).unwrap(),
                all_keys[0].to_string()
            );

            let loaded_bls = keychain
                .load(KeyName::Bls(encode_address(&bls.bls_address().unwrap()).unwrap()), "testpws")
                .unwrap();
            assert_eq!(bls, loaded_bls);
        })
        .await;
    }

    #[tokio::test]
    async fn test_multiple_keys_gen() {
        build_test_dir("keychain_multi", |test_path| async move {
            let keychain = Keychain::new(test_path);

            _ = keychain.generate(KeyType::Bls, Some("mybls"), "testpws");
            _ = keychain.generate(KeyType::Ecdsa, Some("myecdsa"), "testpws");
            let all_keys = keychain.list().unwrap();
            assert_eq!(all_keys.len(), 2);
            for key in all_keys {
                match key {
                    KeyName::Bls(n) => assert_eq!("mybls", &n),
                    KeyName::Ecdsa(n) => assert_eq!("myecdsa", &n),
                }
            }
        })
        .await;
    }

    #[tokio::test]
    async fn test_public_keys() {
        build_test_dir("public_keys", |test_path| async move {
            let keychain = Keychain::new(test_path);

            let ecdsakey = keychain.generate(KeyType::Ecdsa, Some("myecdsa"), "testpws");
            let blskey = keychain.generate(KeyType::Bls, Some("mybls"), "testpws");

            for key in [ecdsakey, blskey] {
                match key.address() {
                    KeyAddress::Ecdsa(addr) => assert_eq!(
                        format!("0x{}", encode(addr.as_bytes())),
                        keychain.public_address(&KeyName::Ecdsa("myecdsa".to_string())).unwrap()
                    ),
                    KeyAddress::Bls(addr) => assert_eq!(
                        serde_json::to_string(&addr).unwrap().trim_matches('"'),
                        keychain.public_address(&KeyName::Bls("mybls".to_string())).unwrap()
                    ),
                }
            }
        })
        .await;
    }
}
