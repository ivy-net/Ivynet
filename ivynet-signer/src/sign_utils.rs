use ethers::{
    abi::{encode, Token},
    types::{Address, Signature, H256, U256},
    utils::keccak256,
};
use ivynet_grpc::messages::{Metrics, NodeData};

use crate::IvyWallet;

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

// --- NodeData ---
pub fn sign_node_data(
    node_data: &NodeData,
    wallet: &IvyWallet,
) -> Result<Signature, IvySigningError> {
    sign_hash(hash_node_data(node_data)?, wallet)
}

fn hash_node_data(node_data: &NodeData) -> Result<H256, IvySigningError> {
    let mut tokens = Vec::new();
    let node_data = node_data.clone();
    tokens.push(Token::String(node_data.name));
    tokens.push(Token::String(node_data.node_type));
    tokens.push(Token::String(node_data.manifest));
    tokens.push(Token::Bool(node_data.metrics_alive));
    Ok(H256::from(&keccak256(encode(&tokens))))
}

pub async fn recover_node_data(
    node_data: &NodeData,
    signature: &Signature,
) -> Result<Address, IvySigningError> {
    recover_from_hash(hash_node_data(node_data)?, signature)
}

// --- Metrics ---
pub fn sign_metrics(metrics: &[Metrics], wallet: &IvyWallet) -> Result<Signature, IvySigningError> {
    sign_hash(build_metrics_message(metrics), wallet)
}

pub async fn recover_metrics(
    metrics: &[Metrics],
    signature: &Signature,
) -> Result<Address, IvySigningError> {
    recover_from_hash(build_metrics_message(metrics), signature)
}

fn build_metrics_message(metrics: &[Metrics]) -> H256 {
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
    H256::from(&keccak256(encode(&tokens)))
}

// --- NameChange ---
pub fn sign_name_change(
    old_name: &str,
    new_name: &str,
    wallet: &IvyWallet,
) -> Result<Signature, IvySigningError> {
    sign_hash(hash_name_change(old_name, new_name)?, wallet)
}

fn hash_name_change(old_name: &str, new_name: &str) -> Result<H256, IvySigningError> {
    let tokens = vec![Token::String(old_name.to_string()), Token::String(new_name.to_string())];
    Ok(H256::from(&keccak256(encode(&tokens))))
}

pub async fn recover_name_change(
    old_name: &str,
    new_name: &str,
    signature: &Signature,
) -> Result<Address, IvySigningError> {
    recover_from_hash(hash_name_change(old_name, new_name)?, signature)
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
