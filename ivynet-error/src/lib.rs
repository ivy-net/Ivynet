pub mod error;

pub use ethers;

use ethers::{
    middleware::{signer::SignerMiddlewareError, SignerMiddleware},
    providers::{Http, Provider},
};
use ivynet_signer::IvyWallet;

pub type IvyProvider = SignerMiddleware<Provider<Http>, IvyWallet>;
pub type IvyProviderError = SignerMiddlewareError<Provider<Http>, IvyWallet>;
