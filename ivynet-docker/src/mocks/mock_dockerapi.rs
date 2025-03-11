use std::{collections::HashMap, pin::Pin, sync::Arc};

use async_trait::async_trait;
use bollard::{
    container::LogOutput,
    errors::Error,
    secret::{ContainerSummary, EventMessage, ImageSummary},
    Docker,
};
use futures::{stream, Stream};

use crate::{
    container::{ContainerId, ContainerImage, FullContainer},
    dockerapi::{DockerApi, DockerClient},
};

// #[derive(Clone)]
// pub struct MockDockerClient {
//     pub records: Vec<ContainerRecord>,
//     pub events: Vec<EventMessage>,
//     pub logs: Vec<LogOutput>,
//     pub images: Arc<Vec<ImageSummary>>,
// }
//
// impl MockDockerClient {
//     pub fn new() -> Self {
//         let records = vec![postgres_container(), memcached_container(), eigenda_container_1()];
//         let events = vec![eigenda_stream_start(), eigenda_stream_down(),
// eigenda_stream_restart()]             .into_iter()
//             .flatten()
//             .collect();
//         let logs = mock_logs();
//         Self { records, events, logs, images: Arc::new(vec![]) }
//     }
//
//     pub fn images_only(&self, images: Vec<ImageSummary>) -> MockDockerClient {
//         MockDockerClient { images: Arc::new(images), records: vec![], events: vec![], logs:
// vec![] }     }
// }
//
// impl Default for MockDockerClient {
//     fn default() -> Self {
//         Self::new()
//     }
// }

// #[async_trait]
// impl DockerApi for MockDockerClient {
//     async fn list_containers(&self) -> Vec<FullContainer> {
//         self.records.iter().map(|r| FullContainer::new(r.container_summary.clone())).collect()
//     }
//
//     fn inner(&self) -> Docker {
//         DockerClient::default().0
//     }
//
//     async fn list_images(&self) -> HashMap<ContainerId, ContainerImage> {
//         DockerClient::process_images(self.images.to_vec())
//     }
//
//     async fn stream_logs(
//         &self,
//         _container: Container,
//         _since: i64,
//     ) -> Pin<Box<dyn Stream<Item = Result<LogOutput, Error>> + Send + Unpin>> {
//         Box::pin(stream::iter(self.logs.clone().into_iter().map(Ok)))
//     }
//
//     async fn stream_logs_by_container_id(
//         &self,
//         _container_id: &str,
//         _since: i64,
//     ) -> Pin<Box<dyn Stream<Item = Result<LogOutput, Error>> + Send + Unpin>> {
//         Box::pin(stream::iter(self.logs.clone().into_iter().map(Ok)))
//     }
//
//     async fn stream_events(
//         &self,
//     ) -> Pin<Box<dyn Stream<Item = Result<EventMessage, Error>> + Send + Unpin>> {
//         Box::pin(stream::iter(self.events.clone().into_iter().map(Ok)))
//     }
// }
//
// // Extra scenarios for testing
// impl MockDockerClient {
//     #[allow(dead_code)]
//     fn double_start_event(&mut self) {
//         self.events.extend(eigenda_stream_start());
//     }
//     #[allow(dead_code)]
//     fn double_down_event(&mut self) {
//         self.events.extend(eigenda_stream_down());
//     }
//     #[allow(dead_code)]
//     fn double_restart_event(&mut self) {
//         self.events.extend(eigenda_stream_restart());
//     }
// }
//
// #[derive(Debug, Clone)]
// pub struct ContainerRecord {
//     _image_name: String,
//     _image_digest: String,
//     container_summary: ContainerSummary,
// }
//
// fn mock_logs() -> Vec<LogOutput> {
//     let stdouts = [
//         "INFO STD OUT TEST The oil must leak",
//         "WARN STD OUT TEST The tongue must toil",
//         "ERROR STD OUT TEST Man must use",
//         "DEBUG STD OUT TEST Both tongue and oil",
//     ]
//     .iter()
//     .map(|s| LogOutput::StdOut { message: s.as_bytes().into() })
//     .collect::<Vec<LogOutput>>();
//
//     let stderr =
//         LogOutput::StdErr { message: "STD ERR LOG TEST: I am the eggman".as_bytes().into() };
//     let stdin = LogOutput::StdIn { message: "STD IN LOG TEST: I am the walrus".as_bytes().into()
// };     let console =
//         LogOutput::Console { message: "CONSOLE LOG TEST: I am the walrus".as_bytes().into() };
//
//     // Flatten them all into one Vec<Result<LogOutput, Error>>:
//     let flattened: Vec<LogOutput> = stdouts
//         .into_iter()
//         .chain(std::iter::once(stderr))
//         .chain(std::iter::once(stdin))
//         .chain(std::iter::once(console))
//         .collect();
//     flattened
// }
//
// #[allow(dead_code)]
// fn mock_non_utf8_logs() -> Vec<LogOutput> {
//     // build several byte arrays that are not valid utf8
//     let msg1 = LogOutput::StdOut { message: vec![0x80, 0x81, 0x82, 0x83, 0x84, 0x85].into() };
//     let msg2 = LogOutput::StdOut { message: vec![0x86, 0x87, 0x88, 0x89, 0x8A, 0x8B].into() };
//     let msg3 = LogOutput::StdOut { message: vec![0x8C, 0x8D, 0x8E, 0x8F, 0x90, 0x91].into() };
//     let msg4 = LogOutput::StdOut { message: vec![0x92, 0x93, 0x94, 0x95, 0x96, 0x97].into() };
//     vec![msg1, msg2, msg3, msg4]
// }
//
// fn postgres_container() -> ContainerRecord {
//     ContainerRecord {
//         _image_name: "postgres:latest".to_string(),
//         _image_digest: "sha256:994cc3113ce004ae73df11f0dbc5088cbe6bb0da1691dd7e6f55474202a4f211"
//             .to_string(),
//         container_summary: serde_json::from_str(include_str!(
//             "./containersummaries/postgres_container_summary.json"
//         ))
//         .unwrap(),
//     }
// }
//
// fn memcached_container() -> ContainerRecord {
//     ContainerRecord {
//         _image_name: "memcached:latest".to_string(),
//         _image_digest: "sha256:706d1761d9646b9f827f049a71fdab99457f90b920c1cca9fc295821b6df1753"
//             .to_string(),
//         container_summary: serde_json::from_str(include_str!(
//             "./containersummaries/memcached_container_summary.json"
//         ))
//         .unwrap(),
//     }
// }
//
// fn eigenda_container_1() -> ContainerRecord {
//     ContainerRecord {
//         _image_name: "ghcr.io/layr-labs/eigenda/opr-node:0.8.4".to_string(),
//         _image_digest: "sha256:6650119a385f2447ca60f03080f381cf4f10ad7f920a2ce27fe0d973ac43e993"
//             .to_string(),
//         container_summary: serde_json::from_str(include_str!(
//             "./containersummaries/eigenda_container_summary_1.json"
//         ))
//         .unwrap(),
//     }
// }
//
// // streams
// fn eigenda_stream_start() -> Vec<EventMessage> {
//     serde_json::from_str(include_str!("./eventstream/eigenda_container_start.json")).unwrap()
// }
//
// fn eigenda_stream_down() -> Vec<EventMessage> {
//     serde_json::from_str(include_str!("./eventstream/eigenda_container_down.json")).unwrap()
// }
//
// fn eigenda_stream_restart() -> Vec<EventMessage> {
//     serde_json::from_str(include_str!("./eventstream/eigenda_container_restart.json")).unwrap()
// }
//
// #[test]
// fn test_load_summaries() {
//     postgres_container();
//     memcached_container();
// }
//
// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     // --- List Image Tests ---
//     #[tokio::test]
//     async fn test_list_images_normal_case() {
//         let mock = MockDockerClient::new();
//         let mock = mock.images_only(vec![ImageSummary {
//             id: "sha256:15b900c8b655dbdb56b1ee66c754d618d4f35551ad8d577b6fee2680b71e1a4d"
//                 .to_string(),
//             repo_tags: vec!["image:latest".to_string()],
//             repo_digests: vec![
//                 "image@sha256:15b900c8b655dbdb56b1ee66c754d618d4f35551ad8d577b6fee2680b71e1a4d"
//                     .to_string(),
//             ],
//             ..Default::default()
//         }]);
//
//         let result = mock.list_images().await;
//
//         assert_eq!(
//             result
//                 .get(&ContainerId::from(
//                     "sha256:15b900c8b655dbdb56b1ee66c754d618d4f35551ad8d577b6fee2680b71e1a4d"
//                 ))
//                 .unwrap(),
//             &ContainerImage::from("image:latest")
//         );
//         assert_eq!(result.len(), 1);
//     }
//
//     #[tokio::test]
//     async fn test_list_images_empty_repo_tags() {
//         let mock = MockDockerClient::new();
//         let mock = mock.images_only(vec![ImageSummary {
//             id: "sha256:15b900c8b655dbdb56b1ee66c754d618d4f35551ad8d577b6fee2680b71e1a4d"
//                 .to_string(),
//             repo_tags: vec![],
//             repo_digests: vec![
//                 "image1@sha256:15b900c8b655dbdb56b1ee66c754d618d4f35551ad8d577b6fee2680b71e1a4d"
//                     .to_string(),
//             ],
//             ..Default::default()
//         }]);
//
//         let result = mock.list_images().await;
//
//         assert_eq!(
//             result
//                 .get(&ContainerId::from(
//                     "sha256:15b900c8b655dbdb56b1ee66c754d618d4f35551ad8d577b6fee2680b71e1a4d"
//                 ))
//                 .unwrap(),
//             &ContainerImage::from("image1")
//         );
//         assert_eq!(result.len(), 1);
//     }
//
//     #[tokio::test]
//     async fn test_list_images_empty_repo_digests() {
//         let mock = MockDockerClient::new();
//         let mock = mock.images_only(vec![ImageSummary {
//             id: "sha256:15b900c8b655dbdb56b1ee66c754d618d4f35551ad8d577b6fee2680b71e1a4d"
//                 .to_string(),
//             repo_tags: vec!["image:latest".to_string()],
//             repo_digests: vec![],
//             ..Default::default()
//         }]);
//
//         let result = mock.list_images().await;
//
//         assert_eq!(
//             result
//                 .get(&ContainerId::from(
//                     "sha256:15b900c8b655dbdb56b1ee66c754d618d4f35551ad8d577b6fee2680b71e1a4d"
//                 ))
//                 .unwrap(),
//             &ContainerImage::from("image:latest")
//         );
//         assert_eq!(result.len(), 1);
//     }
//
//     #[tokio::test]
//     async fn test_list_images_multiple_tags() {
//         let mock = MockDockerClient::new();
//         let mock = mock.images_only(vec![ImageSummary {
//             id: "sha256:bd6936138442b3cf77aab8394fcf054ff70259276eb343feec1edf8f0d06a98c"
//                 .to_string(),
//             repo_tags: vec!["image:latest".to_string(), "image:v1".to_string()],
//             repo_digests: vec![
//                 "image@sha256:bd6936138442b3cf77aab8394fcf054ff70259276eb343feec1edf8f0d06a98c"
//                     .to_string(),
//             ],
//             ..Default::default()
//         }]);
//
//         let result = mock.list_images().await;
//
//         assert_eq!(
//             result
//                 .get(&ContainerId::from(
//                     "sha256:bd6936138442b3cf77aab8394fcf054ff70259276eb343feec1edf8f0d06a98c"
//                 ))
//                 .unwrap(),
//             &ContainerImage::from("image:v1")
//         );
//         assert_eq!(result.len(), 1);
//     }
//
//     #[tokio::test]
//     async fn test_images_broken_empty_list() {
//         let mock = MockDockerClient::new();
//         let mock = mock.images_only(vec![
//             ImageSummary {
//                 id: "sha256:bd6936138442b3cf77aab8394fcf054ff70259276eb343feec1edf8f0d06a98c"
//                     .to_string(),
//                 ..Default::default()
//             },
//             ImageSummary {
//                 id: "sha256:0ddb7a14d16cdc41a73ef2fc4965345661eb4336cf63024a94d7aecc6b36f3c7"
//                     .to_string(),
//                 ..Default::default()
//             },
//         ]);
//         let result = mock.list_images().await;
//         assert_eq!(result.len(), 0);
//     }
//
//     // --- Container Tests ---
//     #[tokio::test]
//     async fn test_list_containers_normal_case() {
//         let mock = MockDockerClient::new();
//         let result = mock.list_containers().await;
//         assert_eq!(result.len(), 3);
//     }
//
//     #[tokio::test]
//     async fn test_list_containers_empty_list() {
//         let mock = MockDockerClient::new();
//         let mock = mock.images_only(vec![]);
//         let result = mock.list_containers().await;
//         assert_eq!(result.len(), 0);
//     }
// }
