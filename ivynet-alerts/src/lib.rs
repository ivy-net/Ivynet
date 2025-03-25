mod alert_flags;
mod alert_type;
mod bitflag;

pub use alert_flags::AlertFlags;
pub use alert_type::{Alert, AlertType};
pub use bitflag::BitflagError;

use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize, sqlx::Type)]
#[sqlx(type_name = "send_state")]
pub enum SendState {
    #[sqlx(rename = "no_send")]
    NoSend,
    #[sqlx(rename = "send_success")]
    SendSuccess,
    #[sqlx(rename = "send_failed")]
    SendFailed,
}
