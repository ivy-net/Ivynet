use self::{
    config::WitnessConfig,
    contracts::{witness_hub_abi::SignatureWithSaltAndExpiry, AvsDirectory, WitnessHub},
};
use super::AvsVariant;
use crate::{
    avs::witness::{contracts::OperatorRegistry, run_config::RunConfig},
    config::IvyConfig,
    eigen::quorum::QuorumType,
    error::IvyError,
    io::{write_json, IoError},
    keys::keyfile::{prompt_ecdsa_keyfile, EcdsaKeyfile},
    rpc_management::IvyProvider,
};
use async_trait::async_trait;
use ethers::{
    contract::ContractError,
    providers::{JsonRpcError, Middleware, MiddlewareError as _, ProviderError},
    signers::WalletError,
    types::{Address, BlockNumber, Bytes, Chain, Signature, H160, H256, U256},
};
use std::{
    path::PathBuf,
    process::{Child, Command},
    sync::Arc,
};
use thiserror::Error as ThisError;
use tracing::{debug, info};

pub mod config;
pub mod contracts;
pub mod run_config;

/// LIGHT NODE ONLY implemented at the moment
///
/// Holesky setup: https://docs.witnesschain.com/rollup-watchtower-network-live/for-the-node-operators/watchtower-setup/holesky-setup
/// Holesky L2 Archive node setup: https://docs.witnesschain.com/rollup-watchtower-network-live/for-the-node-operators/watchtower-setup/holesky-setup/l2-archive-node-setup-guide#light-configuration-for-l2-node-setup
/// Requires whitelist: https://docs.witnesschain.com/rollup-watchtower-network-live/for-the-node-operators/watchtower-setup/holesky-setup

/**
*   Witnesschain registration and setup requirements
*
*   Watchtower operator ECDSA key must also be registerd as an Eigenlayer Operator
*
* - Get whitelisted for Eigenlayer
* - Register Watchtower
*       https://github.com/witnesschain-com/operator-cli/blob/development/watchtower-operator/commands/register_watchtower.go
* - Register Operator to Avs
*       https://github.com/witnesschain-com/operator-cli/blob/development/watchtower-operator/commands/register_op_to_avs.go
*
*/

const WITNESS_PATH: &str = ".eigenlayer/witness";
const ONE_MONTH: u64 = 60 * 60 * 24 * 30;

#[derive(Debug, Clone)]
pub struct Witness {
    path: PathBuf,
    chain: Chain,
    running: bool,
    config: WitnessConfig,
}

impl Witness {
    pub fn new(path: PathBuf, chain: Chain, config: WitnessConfig) -> Self {
        Self { path, chain, running: false, config }
    }

    /// Create a new Witness instance with a default path of $HOME/.eigenlayer/witness and the
    /// default witnessconfig.
    pub fn new_from_chain(chain: Chain) -> Self {
        let home_dir = dirs::home_dir().unwrap();
        let config = WitnessConfig::load_from_default_path().unwrap();
        Self::new(home_dir.join(WITNESS_PATH), chain, config)
    }

    pub fn base_path(&self) -> &PathBuf {
        &self.path
    }

    /// Register for all steps on the Witness Chain Watchtower Network. Required before running the AVS.
    pub async fn register_all(&self, provider: Arc<IvyProvider>) -> Result<(), WitnessError> {
        let operator_address = provider.address();
        let watchtower_address = self
            .config
            .watchtower_ecdsa_file
            .as_ref()
            .ok_or_else(|| WitnessError::CustomError("No watchtower keyfile found".to_owned()))?
            .address;
        if !self.is_operator_whitelisted(provider.clone()).await? {
            return Err(WitnessError::NotWhitelistedError(format!("{:?}", operator_address)));
        }
        if !self.is_watchtower_registered(provider.clone(), watchtower_address).await? {
            self.register_watchtower(provider.clone()).await?;
        }
        if !self.is_operator_registered(provider.clone()).await? {
            self.register_operator_to_avs(provider.clone()).await?;
        }
        Ok(())
    }

    /**
     *
     * CONTRACT FUNCTIONS
     *
     */

    // TODO: Expiry is currently hardcoded to one_month. Make configurable from config.
    /// Register watchtower. Requires watchtower private key stuff. Figure that out.
    pub async fn register_watchtower(
        &self,
        provider: Arc<IvyProvider>,
    ) -> Result<(), WitnessError> {
        // TODO: Default is currently set to one month. Create option for override.
        let operator_address = provider.address();
        let watchtower_address = self
            .config
            .watchtower_ecdsa_file
            .as_ref()
            .ok_or_else(|| WitnessError::CustomError("No watchtower keyfile found".to_owned()))?
            .address;

        let operator_registry =
            OperatorRegistry::new(contracts::operator_registry(self.chain)?, provider.clone());

        if !self.is_operator_whitelisted(provider.clone()).await? {
            return Err(WitnessError::NotWhitelistedError(format!("{:?}", operator_address)));
        }
        if self.is_watchtower_registered(provider.clone(), watchtower_address).await? {
            return Err(WitnessError::AlreadyRegisteredError(format!("{:?}", operator_address)));
        }
        let salt = self.generate_salt();
        let expiry = self.get_expiry_timestamp(provider.clone(), ONE_MONTH).await?;
        let signed_msg: [u8; 65] = self.sign_operator_address(provider, salt, expiry).await?.into();
        let tx_receipt = operator_registry
            .register_watchtower_as_operator(watchtower_address, salt, expiry, signed_msg.into())
            .send()
            .await?
            .await?;
        if let Some(receipt) = tx_receipt {
            if receipt.status == Some(1u64.into()) {
                info!("Watchtower registered, tx hash: {:?}", receipt.transaction_hash);
                return Ok(());
            } else {
                return Err(WitnessError::UnknownContractError);
            }
        }
        Err(WitnessError::UnknownContractError)
    }

    pub async fn deregister_watchtower(
        &self,
        provider: Arc<IvyProvider>,
    ) -> Result<(), WitnessError> {
        let operator_address = provider.address();
        let watchtower_address = self
            .config
            .watchtower_ecdsa_file
            .as_ref()
            .ok_or_else(|| WitnessError::CustomError("No watchtower keyfile found".to_owned()))?
            .address;

        let operator_registry =
            OperatorRegistry::new(contracts::operator_registry(self.chain)?, provider.clone());

        if !self.is_operator_whitelisted(provider.clone()).await? {
            return Err(WitnessError::NotWhitelistedError(format!("{:?}", operator_address)));
        }
        if !self.is_watchtower_registered(provider.clone(), watchtower_address).await? {
            return Err(WitnessError::NotRegisteredError(format!("{:?}", operator_address)));
        }
        let tx_receipt = operator_registry.de_register(watchtower_address).send().await?.await?;
        if let Some(receipt) = tx_receipt {
            if receipt.status == Some(1u64.into()) {
                info!("Watchtower deregistered, tx hash: {:?}", receipt.transaction_hash);
                return Ok(());
            } else {
                return Err(WitnessError::UnknownContractError);
            }
        }
        Err(WitnessError::UnknownContractError)
    }

    // TODO: Expiry as configurable
    pub async fn register_operator_to_avs(
        &self,
        provider: Arc<IvyProvider>,
    ) -> Result<(), WitnessError> {
        let operator_address = provider.address();
        let witness_hub_address = contracts::witness_hub(self.chain)?;

        if !self.is_operator_whitelisted(provider.clone()).await? {
            return Err(WitnessError::NotWhitelistedError(format!("{:?}", operator_address)));
        }
        if !self.is_operator_registered(provider.clone()).await? {
            return Err(WitnessError::AlreadyRegisteredError(format!("{:?}", operator_address)));
        }
        let witness_hub = WitnessHub::new(witness_hub_address, provider.clone());
        let avs_directory =
            AvsDirectory::new(contracts::avs_directory(self.chain)?, provider.clone());

        // Operator signature construction
        let salt = self.generate_salt();
        let expiry = self.get_expiry_timestamp(provider.clone(), ONE_MONTH).await?;
        let digest_hash: [u8; 32] = avs_directory
            .calculate_operator_avs_registration_digest_hash(
                operator_address,
                witness_hub_address,
                salt,
                expiry,
            )
            .await?;
        let signed_msg: [u8; 65] = provider.signer().sign_hash(H256::from(digest_hash))?.into();

        let sig_with_data: SignatureWithSaltAndExpiry =
            SignatureWithSaltAndExpiry { signature: signed_msg.into(), salt, expiry };

        let tx_receipt = witness_hub
            .register_operator_to_avs(operator_address, sig_with_data)
            .send()
            .await?
            .await?;
        if let Some(receipt) = tx_receipt {
            if receipt.status == Some(1u64.into()) {
                info!("Operator registered to AVS, tx hash: {:?}", receipt.transaction_hash);
                return Ok(());
            } else {
                return Err(WitnessError::UnknownContractError);
            }
        }
        Err(WitnessError::UnknownContractError)
    }

    pub async fn deregister_operator_from_avs(
        &self,
        provider: Arc<IvyProvider>,
    ) -> Result<(), WitnessError> {
        let operator_address = provider.address();

        if !self.is_operator_whitelisted(provider.clone()).await? {
            return Err(WitnessError::NotWhitelistedError(format!("{:?}", operator_address)));
        }
        if !self.is_operator_registered(provider.clone()).await? {
            return Err(WitnessError::NotRegisteredError(format!("{:?}", operator_address)));
        }
        let witness_hub = WitnessHub::new(contracts::witness_hub(self.chain)?, provider.clone());
        let tx_receipt =
            witness_hub.deregister_operator_from_avs(operator_address).send().await?.await?;
        if let Some(receipt) = tx_receipt {
            if receipt.status == Some(1u64.into()) {
                info!("Operator deregistered from AVS, tx hash: {:?}", receipt.transaction_hash);
                return Ok(());
            } else {
                return Err(WitnessError::UnknownContractError);
            }
        }
        Err(WitnessError::UnknownContractError)
    }

    /// Returns true if the operator is whitelisted for the Witness Chain AVS. Identical to
    /// isActiveOperator method.
    pub async fn is_operator_whitelisted(
        &self,
        provider: Arc<IvyProvider>,
    ) -> Result<bool, WitnessError> {
        let operator_address = provider.address();
        let operator_registry =
            OperatorRegistry::new(contracts::operator_registry(self.chain)?, provider);
        let result = operator_registry.is_whitelisted(operator_address).await?;
        Ok(result)
    }

    pub async fn is_operator_registered(
        &self,
        provider: Arc<IvyProvider>,
    ) -> Result<bool, WitnessError> {
        let operator_address = provider.address();
        let avs_directory = AvsDirectory::new(contracts::avs_directory(self.chain)?, provider);
        let witness_hub_address = contracts::witness_hub(self.chain)?;
        let result =
            avs_directory.avs_operator_status(witness_hub_address, operator_address).await?;
        let result = match result {
            0u8 => false,
            1u8 => true,
            _ => return Err(WitnessError::UnknownContractError),
        };
        Ok(result)
    }

    pub async fn is_watchtower_registered(
        &self,
        provider: Arc<IvyProvider>,
        watchtower_address: Address,
    ) -> Result<bool, WitnessError> {
        let operator_registry =
            OperatorRegistry::new(contracts::operator_registry(self.chain)?, provider);
        let result = operator_registry.is_valid_watchtower(watchtower_address).await?;
        Ok(result)
    }

    async fn sign_operator_address(
        &self,
        provider: Arc<IvyProvider>,
        salt: [u8; 32],
        expiry: U256,
    ) -> Result<Signature, WitnessError> {
        let operator_address = provider.address();
        let operator_registry =
            OperatorRegistry::new(contracts::operator_registry(self.chain)?, provider.clone());
        let digest_hash: [u8; 32] = operator_registry
            .calculate_watchtower_registration_message_hash(operator_address, salt, expiry)
            .await?;
        println!("{:?}", digest_hash);
        let signed = provider.signer().sign_hash(H256::from(digest_hash))?;
        Ok(signed)
    }

    /// Function designed to mimic GenerateSalt from the witnesschain operator-cli. Witnesschain
    /// uses go-crypto rand.go, the internal docs of which are reproduced here:
    ///
    /// Reader is a global, shared instance of a cryptographically
    /// secure random number generator.
    ///
    ///   - On Linux, FreeBSD, Dragonfly, and Solaris, Reader uses getrandom(2)
    ///     if available, and /dev/urandom otherwise.
    ///   - On macOS and iOS, Reader uses arc4random_buf(3).
    ///   - On OpenBSD and NetBSD, Reader uses getentropy(2).
    ///   - On other Unix-like systems, Reader reads from /dev/urandom.
    ///   - On Windows, Reader uses the ProcessPrng API.
    ///   - On js/wasm, Reader uses the Web Crypto API.
    ///   - On wasip1/wasm, Reader uses random_get from wasi_snapshot_preview1.
    ///
    ///   This implementation differs in that it uses the getrandom crate on all platforms, which
    ///   has its own platform-specific internals.
    fn generate_salt(&self) -> [u8; 32] {
        let mut salt: [u8; 32] = [0; 32];
        getrandom::getrandom(&mut salt).unwrap();
        salt
    }

    async fn get_expiry_timestamp(
        &self,
        provider: Arc<IvyProvider>,
        expiry: u64,
    ) -> Result<U256, WitnessError> {
        let maybe_block = provider
            .get_block(BlockNumber::Latest)
            .await
            .map_err(|e| WitnessError::CustomError(e.to_string()))?;
        if let Some(block) = maybe_block {
            let expiry = block.timestamp + expiry;
            Ok(expiry)
        } else {
            Err(WitnessError::CustomError("Could not get block number".to_owned()))
        }
    }
}

impl Default for Witness {
    fn default() -> Self {
        let home_dir = dirs::home_dir().unwrap();
        Self::new(home_dir.join(WITNESS_PATH), Chain::Holesky, todo!())
    }
}

#[async_trait]
impl AvsVariant for Witness {
    async fn setup(
        &self,
        provider: Arc<IvyProvider>,
        config: &IvyConfig,
        operator_password: Option<String>,
    ) -> Result<(), IvyError> {
        let _ = dotenvy::dotenv().unwrap();
        // Setup witnessconfig
        println!("Configuring Witnesschain operator keyfile...");
        let operator_ecdsa_keyfile = prompt_ecdsa_keyfile()?;
        println!("Configuring Witnesschain watchtower keyfile...");
        let watchtower_ecdsa_keyfile = prompt_ecdsa_keyfile()?;
        let path = self.path.join("witness_config.toml");
        let witness_config =
            WitnessConfig::new(path, Some(operator_ecdsa_keyfile), Some(watchtower_ecdsa_keyfile));
        witness_config.store().map_err(WitnessError::from)?;

        // Download watchtower shell script and run to create run files
        download_install(self.path.clone()).map_err(WitnessError::from)?;

        let run_config_path = self.path.join("watchtower.config.json");
        let mut run_config: RunConfig =
            serde_jsonrc::from_str(&std::fs::read_to_string(&run_config_path)?).map_err(|e| {
                IoError::SerdeRcJsonError { source: e, path: run_config_path.display().to_string() }
            })?;

        run_config.private_key = provider.signer().to_private_key();
        // TODO: UNHARDCODE THIS
        run_config.eth_testnet_websocket_url =
            "wss://ethereum-holesky-rpc.publicnode.com".to_string();
        write_json(&run_config_path, &run_config)?;

        Ok(())
    }

    // TODO: This method may need to be abstracted in some way, as not all AVS types encforce
    // quorum_pericentage.
    fn validate_node_size(&self, quorum_percentage: U256) -> Result<bool, IvyError> {
        todo!()
    }

    //TODO: We may be able to move this to a contract call directly
    async fn register(
        &self,
        provider: Arc<IvyProvider>,
        eigen_path: PathBuf,
        private_keyfile: PathBuf,
        keyfile_password: &str,
    ) -> Result<(), IvyError> {
        todo!()
    }

    async fn unregister(
        &self,
        provider: Arc<IvyProvider>,
        eigen_path: PathBuf,
        private_keyfile: PathBuf,
        keyfile_password: &str,
    ) -> Result<(), IvyError> {
        todo!()
    }

    async fn start(&mut self) -> Result<Child, IvyError> {
        // set current directory
        std::env::set_current_dir(&self.path).map_err(|e| WitnessError::IoError(e))?;
        let child = Command::new("sh")
            .current_dir(&self.path)
            .arg("-c")
            .arg("./docker-run.sh")
            .spawn()
            .map_err(|e| WitnessError::ScriptError(e.to_string()))?;
        Ok(child)
    }

    // TODO: Remove quorums from stop  method if not needed
    async fn stop(&mut self) -> Result<(), IvyError> {
        todo!()
    }

    fn path(&self) -> PathBuf {
        self.path.clone()
    }

    fn running(&self) -> bool {
        self.running
    }

    fn name(&self) -> &'static str {
        "eigenda"
    }
}

fn download_install(witness_path: impl Into<PathBuf>) -> Result<(), WitnessError> {
    debug!("Downloading watchtower installer");
    let _ = Command::new("sh")
        .current_dir(witness_path.into())
        .arg("-c")
        .arg("curl https://witnesschain-com.github.io/install-watchtower-testnet | sh")
        .output()?;
    Ok(())
}

#[derive(ThisError, Debug)]
pub enum WitnessError {
    #[error("Boot script failed: {0}")]
    ScriptError(String),
    #[error("Failed to download resource: {0}")]
    DownloadFailedError(String),
    #[error("Unsupported chain: {0}")]
    UnsupportedChainError(String),
    #[error("Operator not whitelisted: {0}")]
    NotWhitelistedError(String),
    #[error("Account already registered: {0}")]
    AlreadyRegisteredError(String),
    #[error("Account not registered: {0}")]
    NotRegisteredError(String),
    #[error("Contract error: {0}")]
    ContractError(Bytes),
    #[error("Json RPC error: {0}")]
    JsonRpcError(JsonRpcError),
    #[error("Unknown contract error")]
    UnknownContractError,
    #[error("Custom error: {0}")]
    CustomError(String),
    #[error(transparent)]
    WitnessConfigError(#[from] config::WitnessConfigError),
    #[error(transparent)]
    ProviderError(#[from] ProviderError),
    #[error(transparent)]
    WalletError(#[from] WalletError),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    KeyfileError(#[from] crate::keys::keyfile::KeyfileError),
}

impl From<ContractError<IvyProvider>> for WitnessError {
    fn from(value: ContractError<IvyProvider>) -> Self {
        match value {
            ContractError::Revert(bytes) => WitnessError::ContractError(bytes),
            ContractError::MiddlewareError { e } => {
                if let Some(err) = e.as_error_response() {
                    WitnessError::JsonRpcError(err.clone())
                } else {
                    WitnessError::UnknownContractError
                }
            }
            ContractError::ProviderError { e } => {
                if let Some(err) = e.as_error_response() {
                    WitnessError::JsonRpcError(err.clone())
                } else {
                    WitnessError::UnknownContractError
                }
            }
            _ => WitnessError::UnknownContractError,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        avs::witness::contracts::OperatorRegistry, config::IvyConfig, error::IvyError,
        rpc_management::connect_provider, wallet::IvyWallet,
    };
    use dialoguer::Password;
    use ethers::abi::AbiEncode;
    use ethers::types::{Chain, H160, U256};
    use ivynet_macros::h160;
    use std::fs;
    use std::sync::Arc;
    use tempfile::TempDir;

    #[test]
    fn test_download_install() {
        let temp_path = TempDir::new().unwrap();
        let result = download_install(temp_path.path()).expect("Could not get path");
        // print all files + directories inside of temp_path
        for entry in fs::read_dir(temp_path.path()).expect("Could not read directory") {
            let entry = entry.expect("Could not get entry");
            let path = entry.path();
            println!("{:?}", path);
        }
    }

    #[test]
    fn test_generate_salt() {
        let witness = Witness::new_from_chain(Chain::Holesky);
        let salt = witness.generate_salt();
        assert_ne!(salt, [0; 32])
    }

    #[tokio::test]
    async fn test_calculate_watchtower_registration_message_hash() -> Result<(), IvyError> {
        let config = IvyConfig::load_from_default_path()?;
        let rpc = config.holesky_rpc_url;
        let password: String = Password::new()
            .with_prompt("Input the password for your stored ECDSA keyfile")
            .interact()
            .unwrap();
        let wallet =
            Some(IvyWallet::from_keystore(config.default_private_keyfile.clone(), &password)?);
        let provider = Arc::new(connect_provider(&rpc, wallet).await?);
        let operator_address = provider.address();
        let operator_registry =
            OperatorRegistry::new(h160!(0x708CBDDdab358c1fa8efB82c75bB4a116F316Def), provider);

        let salt: [u8; 32] = [
            1, 3, 3, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ];
        let expiry: U256 = 1u64.into();

        let digest_hash: [u8; 32] = operator_registry
            .calculate_watchtower_registration_message_hash(operator_address, salt, expiry)
            .await?;
        let digest_hash = digest_hash.encode_hex();
        println!("{:?}", digest_hash);
        Ok(())
    }

    #[tokio::test]
    async fn test_sign_operator_address() -> Result<(), IvyError> {
        let config = IvyConfig::load_from_default_path()?;
        let rpc = config.holesky_rpc_url;
        let password: String = Password::new()
            .with_prompt("Input the password for your stored ECDSA keyfile")
            .interact()
            .unwrap();
        let wallet =
            Some(IvyWallet::from_keystore(config.default_private_keyfile.clone(), &password)?);
        let provider = Arc::new(connect_provider(&rpc, wallet).await?);

        let salt: [u8; 32] = [
            1, 3, 3, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ];
        let expiry: U256 = 1u64.into();
        let witness = Witness::default();
        let signed = witness.sign_operator_address(provider, salt, expiry).await?;
        let sig: [u8; 65] = signed.into();
        println!("{:?}", sig);

        Ok(())
    }
}
