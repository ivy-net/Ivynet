use std::{collections::HashMap, pin::Pin};

use async_trait::async_trait;
use bollard::{
    container::LogOutput,
    errors::Error,
    secret::{ContainerSummary, EventMessage},
};
use futures::Stream;
use ivynet_node_type::NodeType;

use crate::{container::Container, dockerapi::DockerApi, logs};

#[derive(Clone)]
pub struct MockDockerClient {
    // You can store whatever test data you like here:
    pub containers: Vec<ContainerSummary>,
    pub images: HashMap<String, String>,
}

#[async_trait]
impl DockerApi for MockDockerClient {
    async fn list_containers(&self) -> Vec<ContainerSummary> {
        self.containers.clone()
    }

    async fn list_images(&self) -> HashMap<String, String> {
        self.images.clone()
    }

    // ... and so on for each required method ...
    async fn inspect(&self, image_name: &str) -> Option<Container> {
        None
    }
    async fn inspect_many(&self, image_names: &[&str]) -> Vec<Container> {
        vec![]
    }
    async fn find_container_by_name(&self, name: &str) -> Option<Container> {
        None
    }
    async fn find_node_container(&self, node_type: &NodeType) -> Option<Container> {
        None
    }
    async fn find_node_containers(&self, node_types: &[NodeType]) -> Vec<Container> {
        vec![]
    }
    async fn find_all_node_containers(&self) -> Vec<Container> {
        vec![]
    }

    async fn stream_logs(
        &self,
        container: &Container,
        since: i64,
    ) -> Pin<Box<dyn Stream<Item = Result<LogOutput, Error>> + Send + Unpin>> {
        let log_output = LogOutput {
            message: "Mock log message".to_string(),
            time: 0,
            stream: "stdout".to_string(),
        };
    }

    async fn stream_logs_for_node(
        &self,
        node_type: &NodeType,
        since: i64,
    ) -> Option<Pin<Box<dyn Stream<Item = Result<LogOutput, Error>> + Send>>> {
        todo!()
    }

    async fn stream_logs_for_node_latest(
        &self,
        node_type: &NodeType,
    ) -> Option<Pin<Box<dyn Stream<Item = Result<LogOutput, Error>> + Send>>> {
        todo!()
    }

    async fn stream_logs_for_all_nodes(
        &self,
        since: i64,
    ) -> Pin<Box<dyn Stream<Item = Result<LogOutput, Error>> + Send + Unpin>> {
        todo!()
    }

    async fn stream_logs_for_all_nodes_latest(
        &self,
    ) -> Pin<Box<dyn Stream<Item = Result<LogOutput, Error>> + Send + Unpin>> {
        todo!()
    }

    async fn stream_events(
        &self,
    ) -> Pin<Box<dyn Stream<Item = Result<EventMessage, Error>> + Send + Unpin>> {
        todo!()
    }
}
