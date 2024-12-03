pub mod client;
pub mod ivynet_api;
pub mod server;

pub mod backend {
    tonic::include_proto!("backend");
}

pub mod backend_events {
    tonic::include_proto!("backend_events");
}

pub mod messages {
    tonic::include_proto!("messages");
}

pub use tonic::{self, async_trait, Status};
