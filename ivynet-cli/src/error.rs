use ivynet_core::error::IvyError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    IvyError(#[from] IvyError),

    #[error(transparent)]
    DialoguerError(#[from] dialoguer::Error),

    #[error(transparent)]
    TracingFilterParseError(#[from] tracing_subscriber::filter::ParseError),
}
