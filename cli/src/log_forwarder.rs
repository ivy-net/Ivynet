use std::{fmt, sync::Arc};
use tokio::sync::Mutex;

use ivynet_grpc::{
    backend::backend_client::BackendClient, client::create_channel, tonic::transport::Channel,
};
use tracing::{Event, Subscriber};
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

use crate::{config::IvyConfig, error::Error, ivy_machine::IvyMachine};

pub struct LogForwardingLayer {
    machine: IvyMachine,
    backend: Arc<Mutex<BackendClient<Channel>>>,
}

impl LogForwardingLayer {
    pub async fn from_config(config: &IvyConfig) -> Result<Self, Error> {
        let machine = IvyMachine::from_config(config)?;
        let backend_url = config.get_server_url()?;
        let backend_ca = config.get_server_ca();
        let backend_ca = if backend_ca.is_empty() { None } else { Some(backend_ca) };

        let backend = BackendClient::new(
            create_channel(backend_url, backend_ca).await.expect("Cannot create channel"),
        );
        Ok(Self { machine, backend: Arc::new(Mutex::new(backend)) })
    }
}

impl<S> Layer<S> for LogForwardingLayer
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let formatted = format_event(event);
        let signed_log = match self.machine.sign_client_log(&formatted) {
            Ok(signed_log) => signed_log,
            Err(e) => {
                eprintln!("Error signing log: {:?}", e);
                return;
            }
        };

        let backend = Arc::clone(&self.backend);

        tokio::spawn(async move {
            let mut backend = backend.lock().await;
            // Post no logs to the backend if the backend is not available
            if let Err(e) = backend.client_logs(signed_log).await {
                println!("Failed to send log: {:?}", e);
            }
        });
    }
}

fn format_event(event: &Event) -> String {
    let meta = event.metadata();
    let mut s = format!("Level: {} | Target: {} | ", meta.level(), meta.target());
    let mut visitor = EventVisitor::default();
    event.record(&mut visitor);
    s.push_str(&visitor.output);
    s
}

#[derive(Default)]
struct EventVisitor {
    output: String,
}

impl tracing::field::Visit for EventVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        self.output.push_str(&format!("{}: {:?}; ", field.name(), value));
    }
}
