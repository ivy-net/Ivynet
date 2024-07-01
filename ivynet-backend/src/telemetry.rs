use tracing::Level;
use tracing_subscriber::FmtSubscriber;

use crate::error::BackendError;

pub fn start_tracing(level: Level) -> Result<(), BackendError> {
    let subscriber = FmtSubscriber::builder().with_max_level(level).finish();

    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}
