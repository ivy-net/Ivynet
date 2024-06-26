pub mod client;
pub mod server;

pub mod backend {
    tonic::include_proto!("backend");
}

pub mod messages {
    tonic::include_proto!("messages");
}

pub mod ivy_daemon {
    tonic::include_proto!("ivy_daemon");
}

pub use tonic;
pub use tonic::{async_trait, Status};
