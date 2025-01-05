pub mod client;
pub mod server;

pub mod backend {
    #![allow(clippy::derive_partial_eq_without_eq)]
    tonic::include_proto!("backend");
}

pub mod backend_events {
    #![allow(clippy::derive_partial_eq_without_eq)]
    tonic::include_proto!("backend_events");
}

pub mod messages {
    #![allow(clippy::derive_partial_eq_without_eq)]
    tonic::include_proto!("messages");
}

pub mod database {
    #![allow(clippy::derive_partial_eq_without_eq)]
    tonic::include_proto!("database");
}

pub use tonic::{self, async_trait, Status};
