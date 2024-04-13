use ethers_core::types::{Address, Bytes, U64};
use ethers_providers::{Http, Middleware, Provider, ProviderError, Ws};
use url::Url;

use crate::config;

lazy_static::lazy_static! {
    static ref PROVIDER: Provider<Http> = connect_provider();
}

fn connect_provider() -> Provider<Http> {
    let cfg = config::get_config();
    let url = Url::parse(&cfg.rpc_url).expect("Could not parse saved RPC URL");
    let provider = Provider::new(Http::new(url));

    provider
}

pub async fn get_block() -> Result<(), Box<dyn std::error::Error>> {
    let block = PROVIDER.get_block_number().await?;
    println!("Block number: {:?}", block);
    Ok(())
}

pub async fn get_code() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "0x39053D51B77DC0d36036Fc1fCc8Cb819df8Ef37A".parse::<Address>()?;
    let code = PROVIDER.get_code(addr, None).await?;
    println!("Got code: {}", serde_json::to_string(&code)?);

    Ok(())
}

pub async fn get_operator_details(address: String) -> Result<(), Box<dyn std::error::Error>> {
    let addr = address.parse::<Address>()?;
    // let operator = PROVIDER.get_operator(addr).await?;
    // println!("Operator: {:?}", operator);
    Ok(())
}
