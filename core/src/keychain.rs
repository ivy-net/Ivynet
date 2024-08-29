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

pub enum Key {
    Ecdsa(IvyWallet),
    Bls(BlsKey),
}

pub enum KeyAddress {
    Ecdsa(Address),
    Bls(BlsAddress),
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
        for path in paths {
            if let Ok(file) = path {
                let filename = file.file_name();
                let cmps = filename.to_str().unwrap().split(".").collect::<Vec<&str>>();
                if cmps.len() == 2 {
                    match cmps[1] {
                        "bls" => {
                            if let Ok(address) = decode_address(cmps[0]) {
                                list.push(KeyAddress::Bls(address));
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
            _ = bls.encrypt_and_store(&self.path, format!("{}.bls", address), password.to_string());
        }
        bls
    }

    fn bls_import(&self, key: &str, password: &str) -> Result<BlsKey, IvyError> {
        let bls = BlsKey::from_private_key(key.to_string())?;
        _ = bls.encrypt_and_store(
            &self.path,
            format!("{}.bls", encode_address(&bls.address())?),
            password.to_string(),
        );
        Ok(bls)
    }

    fn bls_load(&self, address: &BlsAddress, password: &str) -> Result<BlsKey, IvyError> {
        Ok(BlsKey::from_keystore(
            self.path.join(format!("{}.bls", encode_address(&address)?)),
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
        Ok(IvyWallet::from_keystore(self.path.join(format!("{:?}.ecdsa", address)), password)?)
    }
}
