use ethers::{
    abi::{encode, Token},
    types::{Address, Signature, H256, U256},
    utils::keccak256,
};

use crate::{error::IvyError, grpc::messages::Metrics, wallet::IvyWallet};

pub async fn sign(metrics: &[Metrics], wallet: &IvyWallet) -> Result<Signature, IvyError> {
    Ok(wallet.signer().sign_hash(build_message(metrics).await?)?)
}

pub async fn recover(metrics: &[Metrics], signature: &Signature) -> Result<Address, IvyError> {
    Ok(signature.recover(build_message(metrics).await?)?)
}

async fn build_message(metrics: &[Metrics]) -> Result<H256, IvyError> {
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
