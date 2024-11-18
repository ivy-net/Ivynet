pub mod avs;
pub mod bls;
pub mod config;
pub mod constants;
pub mod docker;
pub mod download;
pub mod eigen;
pub mod env_parser;
pub mod error;
pub mod grpc;
pub mod io;
pub mod ivy_yaml;
pub mod keychain;
pub mod keys;
pub mod messenger;
pub mod metadata;
pub mod node_type;
pub mod signature;
pub mod system;
pub mod telemetry;
pub mod utils;
pub mod wallet;

pub use blsful::{Bls12381G1Impl, PublicKey, SecretKey};
pub use ethers;

use ethers::{
    middleware::{signer::SignerMiddlewareError, SignerMiddleware},
    providers::{Http, Provider},
};
use wallet::IvyWallet;

pub type IvyProvider = SignerMiddleware<Provider<Http>, IvyWallet>;
pub type IvyProviderError = SignerMiddlewareError<Provider<Http>, IvyWallet>;
