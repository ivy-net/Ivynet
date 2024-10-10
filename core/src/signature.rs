use ethers::{
    abi::{encode, Token},
    types::{Address, Signature, H256, U256},
    utils::keccak256,
};

use crate::{
    error::IvyError,
    grpc::messages::{Metrics, NodeData},
    wallet::IvyWallet,
};

// --- General Signing ---
pub async fn sign_string(string: &str, wallet: &IvyWallet) -> Result<Signature, IvyError> {
    sign_hash(H256::from(&keccak256(encode(&[Token::String(string.to_string())]))), wallet)
}

pub fn sign_hash(hash: H256, wallet: &IvyWallet) -> Result<Signature, IvyError> {
    Ok(wallet.signer().sign_hash(hash)?)
}

pub fn recover_from_hash(hash: H256, signature: &Signature) -> Result<Address, IvyError> {
    Ok(signature.recover(hash)?)
}

pub fn recover_from_string(string: &str, signature: &Signature) -> Result<Address, IvyError> {
    recover_from_hash(
        H256::from(&keccak256(encode(&[Token::String(string.to_string())]))),
        signature,
    )
}

// --- Metrics ---
pub async fn sign_metrics(metrics: &[Metrics], wallet: &IvyWallet) -> Result<Signature, IvyError> {
    sign_hash(build_metrics_message(metrics).await?, wallet)
}

pub async fn recover_metrics(
    metrics: &[Metrics],
    signature: &Signature,
) -> Result<Address, IvyError> {
    recover_from_hash(build_metrics_message(metrics).await?, signature)
}

async fn build_metrics_message(metrics: &[Metrics]) -> Result<H256, IvyError> {
    let mut tokens = Vec::new();
    let mut metrics_vector = metrics.to_vec();
    metrics_vector.sort_by(|a, b| b.name.cmp(&a.name));

    println!("{:#?}", metrics_vector);

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

// --- Node Data ---
pub async fn sign_node_data(data: &[NodeData], wallet: &IvyWallet) -> Result<Signature, IvyError> {
    sign_hash(build_node_data_message(data).await?, wallet)
}

pub async fn recover_node_data(
    data: &[NodeData],
    signature: &Signature,
) -> Result<Address, IvyError> {
    recover_from_hash(build_node_data_message(data).await?, signature)
}

async fn build_node_data_message(data: &[NodeData]) -> Result<H256, IvyError> {
    let mut tokens: Vec<Token> = Vec::new();
    let mut data_vector = data.to_vec();
    println!("{:?}", data_vector);
    // metrics_vector.sort_by(|a, b| b.name.cmp(&a.name));

    // for metric in metrics_vector {
    //     tokens.push(Token::String(metric.name.clone()));
    //     tokens.push(Token::Int(U256::from((metric.value * 1000.0) as u64)));

    //     for attribute in &metric.attributes {
    //         tokens.push(Token::String(attribute.name.clone()));
    //         tokens.push(Token::String(attribute.value.clone()));
    //     }
    // }
    // Ok(H256::from(&keccak256(encode(&tokens))))
    todo!()
}
