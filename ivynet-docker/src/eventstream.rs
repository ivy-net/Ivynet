use bollard::{errors::Error, models::EventMessage};
use tokio_stream::{Stream, StreamExt};
use tracing::error;

#[derive(Debug)]
pub enum DockerEventType {
    Network,
    Container,
    Image,
    Volume,
}

// Type aliases for the event handlers
type ContainerHandler<T> = Box<dyn Fn(&T, &EventMessage)>;

pub struct DockerEventHandler<T> {
    inner: T,
    event_handler: Option<ContainerHandler<T>>,
}

impl<T> DockerEventHandler<T> {
    pub fn new(inner: T) -> Self {
        Self { inner, event_handler: None }
    }

    pub fn on_event<F>(mut self, handler: F) -> Self
    where
        F: Fn(&T, &EventMessage) + 'static,
    {
        self.event_handler = Some(Box::new(handler));
        self
    }
}

pub struct DockerEventStream<T> {
    inner: Box<dyn Stream<Item = Result<EventMessage, Error>> + Unpin>,
    handler: DockerEventHandler<T>,
}

impl<T> DockerEventStream<T> {
    pub fn new(
        stream: impl Stream<Item = Result<EventMessage, Error>> + Unpin + 'static,
        handler: DockerEventHandler<T>,
    ) -> Self {
        Self { inner: Box::new(stream), handler }
    }

    pub async fn start(mut self) {
        while let Some(event) = self.inner.next().await {
            match event {
                Ok(msg) => {
                    if let Some(handler) = &self.handler.event_handler {
                        handler(&self.handler.inner, &msg);
                    }
                }
                Err(e) => error!("Error: {:?}", e),
            }
        }
    }
}
