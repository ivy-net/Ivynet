use crate::error::Result;
use ethers::{
    contract::{abigen, parse_log, ContractError, LogMeta},
    providers::{Middleware, Provider, Ws},
    types::{Address, Chain},
};
use futures::{stream::SelectAll, Stream, StreamExt};
use ivynet_core::{
    directory::get_all_directories_for_chain,
    grpc::{
        backend_events::{
            backend_events_client::BackendEventsClient, LatestBlockRequest, MetadataUriEvent,
            RegistrationEvent,
        },
        tonic::{transport::Channel, Request},
    },
};
use std::{pin::Pin, sync::Arc};
use tokio::sync::Mutex;
use tracing::{debug, info};

const PAGE_SIZE: u64 = 50_000;

pub type EthersStream<'a, T> = Pin<Box<dyn Stream<Item = T> + Send + 'a>>;
abigen!(
    Directory,
    r#"[
        event AVSMetadataURIUpdated(address indexed avs, string metadataURI)
        event OperatorAVSRegistrationStatusUpdated(address indexed operator, address indexed avs, uint8 status)
    ]"#,
);

#[derive(Debug)]
pub enum EthersEvent {
    DirectoryEvent(Box<(DirectoryEvents, LogMeta)>),
    Error,
}

impl From<std::result::Result<(DirectoryEvents, LogMeta), ContractError<Provider<Ws>>>>
    for EthersEvent
{
    fn from(
        value: std::result::Result<(DirectoryEvents, LogMeta), ContractError<Provider<Ws>>>,
    ) -> Self {
        match value {
            Ok(v) => Self::DirectoryEvent(Box::new(v)),
            Err(_) => Self::Error,
        }
    }
}

pub struct CombinedWsPool<'a, T> {
    merge: Arc<Mutex<SelectAll<EthersStream<'a, T>>>>,
    pub backend: BackendEventsClient<Channel>,
    pub client: Arc<Provider<Ws>>,
    pub chain_id: u64,
}

impl<'a, T: 'a> CombinedWsPool<'a, T> {
    pub fn new(
        backend: BackendEventsClient<Channel>,
        client: Arc<Provider<Ws>>,
        chain_id: u64,
    ) -> Self {
        Self { merge: Arc::new(Mutex::new(SelectAll::new())), backend, client, chain_id }
    }

    pub async fn add<A: 'a>(&self, s: EthersStream<'a, A>)
    where
        T: From<A>,
    {
        let mut merge = self.merge.lock().await;

        let new_stream = s.map(T::from);

        merge.push(Box::pin(new_stream) as EthersStream<'a, T>);
    }
}

impl CombinedWsPool<'_, EthersEvent> {
    pub async fn process(&mut self) -> Result<u64> {
        let mut combined_stream = self.merge.lock().await;
        let mut latest_block = 0;
        while let Some(event) = combined_stream.next().await {
            if let EthersEvent::DirectoryEvent(event) = event {
                latest_block =
                    report_directory_event(&mut self.backend, self.chain_id, *event).await?
            }
        }

        debug!("Failing. We need a restart");
        Ok(latest_block)
    }
}

pub async fn fetch(
    rpc_url: &str,
    mut backend: BackendEventsClient<Channel>,
    from_block: u64,
    addresses: &[Address],
) -> Result<()> {
    info!("Starting even listener under {rpc_url}");

    let mut start_block = from_block;
    info!("Blockchain event streaming started from {start_block} block...");

    let provider = Provider::<Ws>::connect_with_reconnects(rpc_url, 0).await?;
    let chain_id = provider.get_chainid().await?.as_u64();
    let last_block = provider.get_block_number().await?.as_u64();
    debug!("Chain id is {chain_id}");
    let addresses = if addresses.is_empty() {
        get_all_directories_for_chain(chain_id.try_into().unwrap_or(Chain::Mainnet))
    } else {
        addresses
    };

    if addresses.is_empty() {
        panic!("No contracts to monitor");
    }

    debug!("We will listen at {addresses:?}");
    start_block = last_block;

    let client = Arc::new(provider.clone());

    let directories =
        addresses.iter().map(|a| Directory::new(*a, client.clone())).collect::<Vec<_>>();

    for dir in &directories {
        let mut last_checked = backend
            .get_latest_block(Request::new(LatestBlockRequest {
                address: dir.address().as_bytes().to_vec(),
                chain_id,
            }))
            .await?
            .into_inner()
            .block_number;
        if last_checked < from_block {
            last_checked = from_block;
        }
        fetch_all_directory_events_between(
            &mut backend,
            &client,
            dir,
            chain_id,
            last_checked,
            last_block,
        )
        .await?;
    }

    let dir_events =
        directories.iter().map(|d| d.events().from_block(start_block)).collect::<Vec<_>>();

    let mut pool: CombinedWsPool<EthersEvent> = CombinedWsPool::new(backend, client, chain_id);

    for dir_event in &dir_events {
        let dir_stream = dir_event.subscribe_with_meta().await?;
        pool.add(Box::pin(dir_stream)
            as EthersStream<
                '_,
                std::result::Result<(DirectoryEvents, LogMeta), ContractError<Provider<Ws>>>,
            >)
        .await;
    }
    info!("Start listening...");

    _ = pool.process().await?;
    Ok(())
}

async fn fetch_all_directory_events_between(
    backend: &mut BackendEventsClient<Channel>,
    provider: &Provider<Ws>,
    dir: &Directory<Provider<Ws>>,
    chain_id: u64,
    start: u64,
    end: u64,
) -> Result<()> {
    let dir_filter = dir.events().filter.from_block(start).to_block(end);
    info!("Fetching directory events between {start} and {end}");
    let mut provider_events = provider.get_logs_paginated(&dir_filter, PAGE_SIZE);
    while let Some(logs) = provider_events.next().await {
        if let Ok(log) = logs {
            let meta = LogMeta::from(&log);
            let event: std::result::Result<DirectoryEvents, ivynet_core::ethers::abi::Error> =
                parse_log(log);
            if let Ok(event) = event {
                _ = report_directory_event(backend, chain_id, (event, meta)).await;
            }
        }
    }

    info!("Successfully fetched all directory events between {start} and {end}");

    Ok(())
}

pub async fn report_directory_event(
    backend: &mut BackendEventsClient<Channel>,
    chain_id: u64,
    event: (DirectoryEvents, LogMeta),
) -> Result<u64> {
    debug!("Reading event {event:?}");

    match event.0 {
        DirectoryEvents::OperatorAVSRegistrationStatusUpdatedFilter(f) => {
            backend
                .report_registration_event(Request::new(RegistrationEvent {
                    directory: event.1.address.as_bytes().to_vec(),
                    avs: f.avs.as_bytes().to_vec(),
                    chain_id,
                    address: f.operator.as_bytes().to_vec(),
                    active: f.status > 0,
                    block_number: event.1.block_number.as_u64(),
                    log_index: event.1.log_index.as_u64(),
                }))
                .await?;
        }
        DirectoryEvents::AvsmetadataURIUpdatedFilter(ev) => {
            info!("AVS metadata URI updated event {ev:?}");

            backend
                .report_metadata_uri_event(Request::new(MetadataUriEvent {
                    avs: ev.avs.as_bytes().to_vec(),
                    metadata_uri: ev.metadata_uri,
                    block_number: event.1.block_number.as_u64(),
                    log_index: event.1.log_index.as_u64(),
                }))
                .await?;
        }
    }

    Ok(event.1.block_number.as_u64())
}
