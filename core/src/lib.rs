pub mod avs;
pub mod constants;
pub mod directory;
pub mod download;
pub mod eigen;
pub mod env_parser;
pub mod error;
pub mod ivy_yaml;
pub mod keys;
pub mod utils;

pub use ethers;

use ethers::{
    middleware::{signer::SignerMiddlewareError, SignerMiddleware},
    providers::{Http, Provider},
};
use ivynet_signer::IvyWallet;

pub type IvyProvider = SignerMiddleware<Provider<Http>, IvyWallet>;
pub type IvyProviderError = SignerMiddlewareError<Provider<Http>, IvyWallet>;
