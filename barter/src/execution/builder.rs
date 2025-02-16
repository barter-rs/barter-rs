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
        mock::{MockExecution, MockExecutionClientConfig, MockExecutionConfig},
        ExecutionClient,
    },
    exchange::mock::{request::MockExchangeRequest, MockExchange},
    indexer::AccountEventIndexer,
    map::generate_execution_instrument_map,
    UnindexedAccountEvent,
};
use barter_instrument::{
    asset::{name::AssetNameExchange, AssetIndex},
    exchange::{ExchangeId, ExchangeIndex},
    index::IndexedInstruments,
    instrument::{
        kind::InstrumentKind,
        name::InstrumentNameExchange,
        spec::{InstrumentSpec, InstrumentSpecQuantity, OrderQuantityUnits},
        Instrument, InstrumentIndex,
    },
    Keyed, Underlying,
};
use barter_integration::channel::{mpsc_unbounded, Channel, UnboundedTx};
use fnv::FnvHashMap;
use futures::Stream;
use std::{future::Future, pin::Pin, sync::Arc, time::Duration};
use tokio::sync::{broadcast, mpsc};

type ExecutionInitFutures = Vec<Pin<Box<dyn Future<Output = Result<(), ExecutionError>>>>>;

/// Builder for adding and initialising [`ExecutionManager`]s.
///
/// Handles:
/// - Initialising mock execution managers (mocks a specific exchange internally via the [`MockExchange`]).
/// - Initialising live execution managers, setting up an external connection to each exchange.
/// - Constructs a [`MultiExchangeTxMap`] with an entry for each mock/live execution manager.
/// - Combines all exchange account streams into a unified [`AccountStreamEvent`] `Stream`.
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
    /// Construct a new `ExecutionBuilder` using the provided `IndexedInstruments`.
    pub fn new(instruments: &'a IndexedInstruments) -> Self {
        Self {
            merged_channel: Channel::default(),
            instruments,
            channels: FnvHashMap::default(),
            futures: Vec::default(),
        }
    }

    /// Adds an [`ExecutionManager`] for a mocked exchange, setting up a [`MockExchange`]
    /// internally.
    ///
    /// The provided [`MockExecutionConfig`] is used to configure the [`MockExchange`] and provide
    /// the initial account state.
    pub fn add_mock(self, config: MockExecutionConfig) -> Result<Self, BarterError> {
        const ACCOUNT_STREAM_CAPACITY: usize = 256;
        const DUMMY_EXECUTION_REQUEST_TIMEOUT: Duration = Duration::from_secs(1);

        let mocked_exchange = config.mocked_exchange;

        let (request_tx, request_rx) = mpsc::unbounded_channel();

        let (event_tx, event_rx) = broadcast::channel(ACCOUNT_STREAM_CAPACITY);

        // Run MockExchange
        self.init_mock_exchange(config, request_rx, event_tx);

        self.add_execution::<MockExecution>(
            mocked_exchange,
            MockExecutionClientConfig {
                mocked_exchange,
                request_tx,
                event_rx,
            },
            DUMMY_EXECUTION_REQUEST_TIMEOUT,
        )
    }

    fn init_mock_exchange(
        &self,
        config: MockExecutionConfig,
        request_rx: mpsc::UnboundedReceiver<MockExchangeRequest>,
        event_tx: broadcast::Sender<UnindexedAccountEvent>,
    ) {
        let instruments =
            generate_mock_exchange_instruments(self.instruments, config.mocked_exchange);

        let exchange = MockExchange::new(config, request_rx, event_tx, instruments);

        tokio::spawn(exchange.run());
    }

    /// Adds an [`ExecutionManager`] for a live exchange.
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
                "ExecutionBuilder does not support duplicate mocked ExecutionManagers: {exchange}"
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

    /// Awaits initialisation of all mock and live [`ExecutionManager`]s added to the
    /// [`ExecutionBuilder`].
    ///
    /// This method:
    /// - Initialises all [`ExecutionManager`]s asynchronously.
    /// - Constructs an indexed [`MultiExchangeTxMap`] containing the execution request transmitters
    ///   for every exchange.
    /// - Combines all exchange account streams into a unified [`AccountStreamEvent`] `Stream`.
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

fn generate_mock_exchange_instruments(
    instruments: &IndexedInstruments,
    exchange: ExchangeId,
) -> FnvHashMap<InstrumentNameExchange, Instrument<ExchangeId, AssetNameExchange>> {
    instruments
        .instruments()
        .iter()
        .filter_map(
            |Keyed {
                 key: _,
                 value: instrument,
             }| {
                if instrument.exchange.value != exchange {
                    return None;
                }

                let Instrument {
                    exchange,
                    name_internal,
                    name_exchange,
                    underlying,
                    quote,
                    kind,
                    spec,
                } = instrument;

                let kind = match kind {
                    InstrumentKind::Spot => InstrumentKind::Spot,
                    unsupported => {
                        panic!("MockExchange does not support: {unsupported:?}")
                    }
                };

                let spec = match spec {
                    Some(spec) => {
                        let InstrumentSpec {
                            price,
                            quantity:
                                InstrumentSpecQuantity {
                                    unit,
                                    min,
                                    increment,
                                },
                            notional,
                        } = spec;

                        let unit = match unit {
                            OrderQuantityUnits::Asset(asset) => {
                                let quantity_asset = instruments
                                    .find_asset(*asset)
                                    .unwrap()
                                    .asset
                                    .name_exchange
                                    .clone();
                                OrderQuantityUnits::Asset(quantity_asset)
                            }
                            OrderQuantityUnits::Contract => OrderQuantityUnits::Contract,
                            OrderQuantityUnits::Quote => OrderQuantityUnits::Quote,
                        };

                        Some(InstrumentSpec {
                            price: *price,
                            quantity: InstrumentSpecQuantity {
                                unit,
                                min: *min,
                                increment: *increment,
                            },
                            notional: *notional,
                        })
                    }
                    None => None,
                };

                let underlying_base = instruments
                    .find_asset(underlying.base)
                    .unwrap()
                    .asset
                    .name_exchange
                    .clone();

                let underlying_quote = instruments
                    .find_asset(underlying.quote)
                    .unwrap()
                    .asset
                    .name_exchange
                    .clone();

                let instrument = Instrument {
                    exchange: exchange.value,
                    name_internal: name_internal.clone(),
                    name_exchange: name_exchange.clone(),
                    underlying: Underlying {
                        base: underlying_base,
                        quote: underlying_quote,
                    },
                    quote: *quote,
                    kind,
                    spec,
                };

                Some((instrument.name_exchange.clone(), instrument))
            },
        )
        .collect()
}
