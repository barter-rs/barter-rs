use crate::{
    engine::execution_tx::MultiExchangeTxMap,
    error::BarterError,
    execution::{
        error::ExecutionError, manager::ExecutionManager, request::ExecutionRequest,
        AccountStreamEvent,
    },
};
use barter_data::streams::{
    consumer::STREAM_RECONNECTION_POLICY, reconnect::stream::ReconnectingStream,
};
use barter_execution::{
    client::{
        mock::{MockExecution, MockExecutionConfig},
        ExecutionClient,
    },
    indexer::AccountEventIndexer,
    map::generate_execution_instrument_map,
};
use barter_instrument::{
    asset::AssetIndex,
    exchange::{ExchangeId, ExchangeIndex},
    index::IndexedInstruments,
    instrument::InstrumentIndex,
};
use barter_integration::channel::{mpsc_unbounded, Channel, UnboundedTx};
use fnv::FnvHashMap;
use futures::Stream;
use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

pub type ExecutionInitFutures = Vec<Pin<Box<dyn Future<Output = Result<(), ExecutionError>>>>>;

#[allow(missing_debug_implementations)]
pub struct ExecutionBuilder<'a> {
    merged_channel: Channel<AccountStreamEvent<ExchangeIndex, AssetIndex, InstrumentIndex>>,
    instruments: &'a IndexedInstruments,
    channels: FnvHashMap<
        ExchangeId,
        (
            ExchangeIndex,
            UnboundedTx<ExecutionRequest<ExchangeIndex, InstrumentIndex>>,
        ),
    >,
    futures: ExecutionInitFutures,
}

impl<'a> ExecutionBuilder<'a> {
    pub fn new(instruments: &'a IndexedInstruments) -> Self {
        Self {
            merged_channel: Channel::default(),
            instruments,
            channels: FnvHashMap::default(),
            futures: Vec::default(),
        }
    }

    pub fn add_mock(self, config: MockExecutionConfig) -> Result<Self, BarterError> {
        const DUMMY_EXECUTION_REQUEST_TIMEOUT: Duration = Duration::from_secs(1);

        self.add_execution::<MockExecution>(
            config.mocked_exchange,
            config,
            DUMMY_EXECUTION_REQUEST_TIMEOUT,
        )
    }

    pub fn add_live<Client>(
        self,
        config: Client::Config,
        request_timeout: Duration,
    ) -> Result<Self, BarterError>
    where
        Client: ExecutionClient + Send + Sync + 'static,
        Client::AccountStream: Send,
    {
        self.add_execution::<Client>(Client::EXCHANGE, config, request_timeout)
    }

    fn add_execution<Client>(
        mut self,
        exchange: ExchangeId,
        config: Client::Config,
        request_timeout: Duration,
    ) -> Result<Self, BarterError>
    where
        Client: ExecutionClient + Send + Sync + 'static,
        Client::AccountStream: Send,
    {
        let instrument_map = generate_execution_instrument_map(self.instruments, exchange)?;

        let (execution_tx, execution_rx) = mpsc_unbounded();

        if self
            .channels
            .insert(exchange, (instrument_map.exchange.key, execution_tx))
            .is_some()
        {
            return Err(BarterError::ExecutionBuilder(format!(
                "ExecutionBuilder does not support multiple mocked ExecutionManagers. Duplicate: {exchange}"
            )));
        }

        let merged_tx = self.merged_channel.tx.clone();

        self.futures.push(Box::pin(async move {
            // Initialise ExecutionManager
            let (manager, account_stream) = ExecutionManager::init(
                execution_rx.into_stream(),
                request_timeout,
                Arc::new(Client::new(config)),
                AccountEventIndexer::new(Arc::new(instrument_map)),
                STREAM_RECONNECTION_POLICY,
            )
            .await?;

            tokio::spawn(manager.run());
            tokio::spawn(account_stream.forward_to(merged_tx));

            Ok(())
        }));

        Ok(self)
    }

    pub async fn init(
        mut self,
    ) -> Result<
        (
            MultiExchangeTxMap<UnboundedTx<ExecutionRequest<ExchangeIndex, InstrumentIndex>>>,
            impl Stream<Item = AccountStreamEvent<ExchangeIndex, AssetIndex, InstrumentIndex>>,
        ),
        BarterError,
    > {
        // Await ExecutionManager::init futures and ensure success
        futures::future::try_join_all(self.futures).await?;

        // Construct indexed ExecutionTx map
        let execution_tx_map = self
            .instruments
            .exchanges()
            .iter()
            .map(|exchange| {
                // If IndexedInstruments execution not used for execution, add None to map
                let Some((added_execution_exchange_index, added_execution_exchange_tx)) =
                    self.channels.remove(&exchange.value)
                else {
                    return (exchange.value, None);
                };

                assert_eq!(
                    exchange.key,
                    added_execution_exchange_index,
                    "execution ExchangeIndex != IndexedInstruments Keyed<ExchangeIndex, ExchangeId>"
                );

                // If execution has been added, add Some(ExecutionTx) to map
                (exchange.value, Some(added_execution_exchange_tx))
            })
            .collect();

        let account_stream = self.merged_channel.rx.into_stream();

        Ok((execution_tx_map, account_stream))
    }
}
