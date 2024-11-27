use ethers::{
    abi::{encode, Token},
    types::{Address, Signature, H256, U256},
    utils::keccak256,
};

use crate::{grpc::messages::Metrics, wallet::IvyWallet};

// --- General Signing ---
pub fn sign_string(string: &str, wallet: &IvyWallet) -> Result<Signature, IvySigningError> {
    sign_hash(H256::from(&keccak256(encode(&[Token::String(string.to_string())]))), wallet)
}

pub fn sign_hash(hash: H256, wallet: &IvyWallet) -> Result<Signature, IvySigningError> {
    Ok(wallet.signer().sign_hash(hash)?)
}

pub fn recover_from_hash(hash: H256, signature: &Signature) -> Result<Address, IvySigningError> {
    Ok(signature.recover(hash)?)
}

pub fn recover_from_string(
    string: &str,
    signature: &Signature,
) -> Result<Address, IvySigningError> {
    recover_from_hash(
        H256::from(&keccak256(encode(&[Token::String(string.to_string())]))),
        signature,
    )
}

// --- Metrics ---
pub fn sign_metrics(metrics: &[Metrics], wallet: &IvyWallet) -> Result<Signature, IvySigningError> {
    sign_hash(build_metrics_message(metrics)?, wallet)
}

pub async fn recover_metrics(
    metrics: &[Metrics],
    signature: &Signature,
) -> Result<Address, IvySigningError> {
    recover_from_hash(build_metrics_message(metrics)?, signature)
}

fn build_metrics_message(metrics: &[Metrics]) -> Result<H256, IvySigningError> {
    let mut tokens = Vec::new();
    let mut metrics_vector = metrics.to_vec();
    metrics_vector.sort_by(|a, b| b.name.cmp(&a.name));

    for metric in metrics_vector {
        tokens.push(Token::String(metric.name.clone()));
        tokens.push(Token::Int(U256::from((metric.value * 1000.0) as u64)));

        for attribute in &metric.attributes {
            tokens.push(Token::String(attribute.name.clone()));
            tokens.push(Token::String(attribute.value.clone()));
        }
    }
    Ok(H256::from(&keccak256(encode(&tokens))))
}

#[derive(Debug, thiserror::Error)]
pub enum IvySigningError {
    #[error("Bls signing error: {0}")]
    SigningError(#[from] crate::bls::BlsKeyError),
    #[error("Wallet signing error: {0}")]
    WalletSigningError(#[from] ethers::signers::WalletError),
    #[error("Signature recovery error: {0}")]
    RecoveryError(#[from] ethers::types::SignatureError),
}

impl From<IvySigningError> for tonic::Status {
    fn from(e: IvySigningError) -> Self {
        Self::from_error(Box::new(e))
    }
}
