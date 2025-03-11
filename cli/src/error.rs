use ivynet_docker::dockerapi::DockerClientError;

use crate::ivy_machine::MachineIdentityError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    ServerError(#[from] ivynet_grpc::server::ServerError),

    #[error(transparent)]
    DialoguerError(#[from] dialoguer::Error),

    #[error(transparent)]
    GlobalTracingSetError(#[from] tracing::subscriber::SetGlobalDefaultError),

    #[error(transparent)]
    GRPCError(#[from] ivynet_grpc::Status),

    #[error(transparent)]
    GRPCClientError(#[from] ivynet_grpc::client::ClientError),

    #[error("No AVS selected for log viewing. Please select an AVS first, or specify the AVS and chain you would like to view logs for.")]
    NoAvsSelectedLogError,

    #[error("Metadata Uri Not Found")]
    MetadataUriNotFoundError,

    #[error(transparent)]
    StdIo(#[from] std::io::Error),

    #[error("Invalid selection")]
    InvalidSelection,

    #[error("No ECDSA key found in your keychain")]
    NoECDSAKey,

    #[error("No BLS key found in your keychain")]
    NoBLSKey,

    #[error("Failed to parse chain: {0}")]
    ChainParseError(String),

    #[error(transparent)]
    ConfigError(#[from] crate::config::ConfigError),

    #[error(transparent)]
    SignerError(#[from] ivynet_signer::IvyWalletError),

    #[error(transparent)]
    KeychainError(#[from] ivynet_signer::keychain::KeychainError),

    #[error("Chain Unimplemented: {0}")]
    ChainUnimplemented(String),

    #[error("Invalid server URI")]
    InvalidUri,

    #[error("Machine Identity Error")]
    MachineIdentityError(#[from] MachineIdentityError),

    #[error(transparent)]
    DockerClientError(#[from] DockerClientError),
}
