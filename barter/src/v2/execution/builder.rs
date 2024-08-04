// ExecutionBuilder:
// - Builds Execution Channels, returning FnvIndexMap<ExchangeId, ExecutionTx>
// - Initialises all ExecutionManagers

use crate::v2::{
    engine::execution_tx::MultiExchangeTxMap,
    error::BarterError,
    execution::{
        error::IndexedExecutionError,
        manager::{client::ExecutionClient, AccountStreamEvent, ExecutionManager},
        ExecutionRequest,
    },
    instrument::IndexedInstruments,
};
use barter_data::streams::{
    consumer::STREAM_RECONNECTION_POLICY, reconnect::stream::ReconnectingStream,
};
use barter_instrument::{
    asset::AssetIndex,
    exchange::{ExchangeId, ExchangeIndex},
    instrument::InstrumentIndex,
};
use barter_integration::channel::{mpsc_unbounded, Channel, Tx, UnboundedTx};
use fnv::FnvHashMap;
use futures::Stream;
use std::{future::Future, pin::Pin};

pub type ExecutionInitFutures =
    Vec<Pin<Box<dyn Future<Output = Result<(), IndexedExecutionError>>>>>;

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

    #[allow(clippy::should_implement_trait)]
    pub fn add<Client>(mut self, config: Client::Config) -> Result<Self, BarterError>
    where
        Client: ExecutionClient + Send + Sync + 'static,
        Client::AccountStream: Send,
    {
        let exchange = Client::EXCHANGE;

        let execution_map = self.instruments.execution_instrument_map(exchange)?;

        let (execution_tx, execution_rx) = mpsc_unbounded();

        if self
            .channels
            .insert(Client::EXCHANGE, (execution_map.exchange, execution_tx))
            .is_some()
        {
            return Err(BarterError::ExecutionBuilder(format!(
               "ExecutionBuilder does not support multiple ExecutionManagers. Duplicate: {exchange}"
            )));
        }

        let merged_tx = self.merged_channel.tx.clone();

        self.futures.push(Box::pin(async move {
            // Initialise ExecutionManager
            let (account_snapshot, manager, account_stream) = ExecutionManager::init(
                execution_rx.into_stream(),
                Client::new(config),
                execution_map,
                STREAM_RECONNECTION_POLICY,
            )
            .await?;

            merged_tx.send(account_snapshot).unwrap();
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
            .exchanges
            .iter()
            .map(|exchange| {
                // If IndexedInstruments exchange not used for execution, add None to map
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
