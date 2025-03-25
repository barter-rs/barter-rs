use crate::{
    engine::execution_tx::MultiExchangeTxMap,
    error::BarterError,
    execution::{
        AccountStreamEvent, error::ExecutionError, manager::ExecutionManager,
        request::ExecutionRequest,
    },
};
use barter_data::streams::{
    consumer::STREAM_RECONNECTION_POLICY, reconnect::stream::ReconnectingStream,
};
use barter_execution::{
    UnindexedAccountEvent,
    client::{
        ExecutionClient,
        mock::{MockExecution, MockExecutionClientConfig, MockExecutionConfig},
    },
    exchange::mock::{MockExchange, request::MockExchangeRequest},
    indexer::AccountEventIndexer,
    map::generate_execution_instrument_map,
};
use barter_instrument::{
    Keyed, Underlying,
    asset::{AssetIndex, name::AssetNameExchange},
    exchange::{ExchangeId, ExchangeIndex},
    index::IndexedInstruments,
    instrument::{
        Instrument, InstrumentIndex,
        kind::InstrumentKind,
        name::InstrumentNameExchange,
        spec::{InstrumentSpec, InstrumentSpecQuantity, OrderQuantityUnits},
    },
};
use barter_integration::channel::{Channel, UnboundedTx, mpsc_unbounded};
use fnv::FnvHashMap;
use std::{pin::Pin, sync::Arc, time::Duration};
use tokio::sync::{broadcast, mpsc};
use tokio_stream::wrappers::UnboundedReceiverStream;

type ExecutionTx = UnboundedTx<ExecutionRequest<ExchangeIndex, InstrumentIndex>>;
type ExecutionInitFuture =
    Pin<Box<dyn Future<Output = Result<(RunFuture, RunFuture), ExecutionError>> + Send>>;
type RunFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

/// Full execution infrastructure builder.
///
/// Add Mock and Live [`ExecutionClient`] configurations and let the builder set up the required
/// infrastructure.
///
/// Once you have added all the configurations, call [`ExecutionBuilder::build`] to return the
/// full [`ExecutionBuild`]. Then calling [`ExecutionBuild::init`] will then initialise
/// the built infrastructure.
///
/// Handles:
/// - Building mock execution managers (mocks a specific exchange internally via the [`MockExchange`]).
/// - Building live execution managers, setting up an external connection to each exchange.
/// - Constructs a [`MultiExchangeTxMap`] with an entry for each mock/live execution manager.
/// - Combines all exchange account streams into a unified [`AccountStreamEvent`] `Stream`.
#[allow(missing_debug_implementations)]
pub struct ExecutionBuilder<'a> {
    instruments: &'a IndexedInstruments,
    execution_txs: FnvHashMap<ExchangeId, (ExchangeIndex, ExecutionTx)>,
    merged_channel: Channel<AccountStreamEvent<ExchangeIndex, AssetIndex, InstrumentIndex>>,
    mock_exchange_futures: Vec<RunFuture>,
    execution_init_futures: Vec<ExecutionInitFuture>,
}

impl<'a> ExecutionBuilder<'a> {
    /// Construct a new `ExecutionBuilder` using the provided `IndexedInstruments`.
    pub fn new(instruments: &'a IndexedInstruments) -> Self {
        Self {
            instruments,
            execution_txs: FnvHashMap::default(),
            merged_channel: Channel::default(),
            mock_exchange_futures: Vec::default(),
            execution_init_futures: Vec::default(),
        }
    }

    /// Adds an [`ExecutionManager`] for a mocked exchange, setting up a [`MockExchange`]
    /// internally.
    ///
    /// The provided [`MockExecutionConfig`] is used to configure the [`MockExchange`] and provide
    /// the initial account state.
    pub fn add_mock(mut self, config: MockExecutionConfig) -> Result<Self, BarterError> {
        const ACCOUNT_STREAM_CAPACITY: usize = 256;
        const DUMMY_EXECUTION_REQUEST_TIMEOUT: Duration = Duration::from_secs(1);

        let (request_tx, request_rx) = mpsc::unbounded_channel();
        let (event_tx, event_rx) = broadcast::channel(ACCOUNT_STREAM_CAPACITY);

        let mock_execution_client_config = MockExecutionClientConfig {
            mocked_exchange: config.mocked_exchange,
            request_tx,
            event_rx,
        };

        // Register MockExchange init Future
        let mock_exchange_future = self.init_mock_exchange(config, request_rx, event_tx);
        self.mock_exchange_futures.push(mock_exchange_future);

        self.add_execution::<MockExecution>(
            mock_execution_client_config.mocked_exchange,
            mock_execution_client_config,
            DUMMY_EXECUTION_REQUEST_TIMEOUT,
        )
    }

    fn init_mock_exchange(
        &self,
        config: MockExecutionConfig,
        request_rx: mpsc::UnboundedReceiver<MockExchangeRequest>,
        event_tx: broadcast::Sender<UnindexedAccountEvent>,
    ) -> RunFuture {
        let instruments =
            generate_mock_exchange_instruments(self.instruments, config.mocked_exchange);
        Box::pin(MockExchange::new(config, request_rx, event_tx, instruments).run())
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
        Client::Config: Send,
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
        Client::Config: Send,
    {
        let instrument_map = generate_execution_instrument_map(self.instruments, exchange)?;

        let (execution_tx, execution_rx) = mpsc_unbounded();

        if self
            .execution_txs
            .insert(exchange, (instrument_map.exchange.key, execution_tx))
            .is_some()
        {
            return Err(BarterError::ExecutionBuilder(format!(
                "ExecutionBuilder does not support duplicate mocked ExecutionManagers: {exchange}"
            )));
        }

        let merged_tx = self.merged_channel.tx.clone();

        self.execution_init_futures.push(Box::pin(async move {
            // Initialise ExecutionManager
            let (manager, account_stream) = ExecutionManager::init(
                execution_rx.into_stream(),
                request_timeout,
                Arc::new(Client::new(config)),
                AccountEventIndexer::new(Arc::new(instrument_map)),
                STREAM_RECONNECTION_POLICY,
            )
            .await?;

            let manager_future: RunFuture = Box::pin(manager.run());
            let stream_future: RunFuture = Box::pin(account_stream.forward_to(merged_tx));

            Ok((manager_future, stream_future))
        }));

        Ok(self)
    }

    /// Consume this `ExecutionBuilder` and build a full [`ExecutionBuild`] containing all the
    /// [`ExecutionManager`] (mock & live) and [`MockExchange`] futures.
    ///
    /// **For most users, calling [`ExecutionBuild::init`] after this is satisfactory.**
    ///
    /// If you want more control over what runtime drives the futures to completion, you can
    /// de-structure the `ExecutionBuild` and handle accordingly.
    pub async fn build(mut self) -> Result<ExecutionBuild, BarterError> {
        // Await ExecutionManager build futures and ensure success
        let manager_run_futures =
            futures::future::try_join_all(self.execution_init_futures).await?;

        // Construct indexed ExecutionTx map
        let execution_tx_map = self
            .instruments
            .exchanges()
            .iter()
            .map(|exchange| {
                // If IndexedInstruments execution not used for execution, add None to map
                let Some((added_execution_exchange_index, added_execution_exchange_tx)) =
                    self.execution_txs.remove(&exchange.value)
                else {
                    return (exchange.value, None);
                };

                assert_eq!(
                    exchange.key, added_execution_exchange_index,
                    "execution ExchangeIndex != IndexedInstruments Keyed<ExchangeIndex, ExchangeId>"
                );

                // If execution has been added, add Some(ExecutionTx) to map
                (exchange.value, Some(added_execution_exchange_tx))
            })
            .collect();

        let account_stream = self.merged_channel.rx.into_stream();

        Ok(ExecutionBuild {
            manager_run_futures,
            mock_exchange_run_futures: self.mock_exchange_futures,
            execution_tx_map,
            account_stream,
        })
    }
}

/// Container holding execution infrastructure components ready to be initialised.
///
/// Call [`ExecutionBuild::init`] to run all the required execution component futures on tokio
/// tasks - returns the [`MultiExchangeTxMap`] and multi-exchange [`AccountStreamEvent`] stream.
#[allow(missing_debug_implementations)]
pub struct ExecutionBuild {
    pub manager_run_futures: Vec<(RunFuture, RunFuture)>,
    pub mock_exchange_run_futures: Vec<RunFuture>,
    pub execution_tx_map: MultiExchangeTxMap<ExecutionTx>,
    pub account_stream: UnboundedReceiverStream<AccountStreamEvent>,
}

impl ExecutionBuild {
    /// Initialise all execution infrastructure components by spawning all futures onto the
    /// current tokio runtime.
    ///
    /// Returns the [`MultiExchangeTxMap`] and multi-exchange [`AccountStreamEvent`] stream.
    pub fn init(
        self,
    ) -> (
        MultiExchangeTxMap,
        UnboundedReceiverStream<AccountStreamEvent>,
    ) {
        self.mock_exchange_run_futures
            .into_iter()
            .for_each(|mock_exchange_run_future| {
                tokio::spawn(mock_exchange_run_future);
            });

        self.manager_run_futures.into_iter().for_each(
            |(manager_run_future, account_event_forward_future)| {
                tokio::spawn(manager_run_future);
                tokio::spawn(account_event_forward_future);
            },
        );

        (self.execution_tx_map, self.account_stream)
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
