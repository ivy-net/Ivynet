pub mod client;
pub mod server;

pub mod backend {
    tonic::include_proto!("backend");
}

pub mod messages {
    tonic::include_proto!("messages");
}

pub use tonic::{self, async_trait, Status};
