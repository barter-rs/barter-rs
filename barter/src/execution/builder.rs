use crate::{
    engine::{clock::EngineClock, execution_tx::MultiExchangeTxMap},
    error::BarterError,
    execution::{
        AccountStreamEvent, Execution, error::ExecutionError, manager::ExecutionManager,
        request::ExecutionRequest,
    },
    shutdown::AsyncShutdown,
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
use futures::{FutureExt, future::try_join_all};
use std::{pin::Pin, sync::Arc, time::Duration};
use tokio::{
    sync::{broadcast, mpsc},
    task::{JoinError, JoinHandle},
};

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
    execution_txs: FnvHashMap<ExchangeId, (ExchangeIndex, UnboundedTx<ExecutionRequest>)>,
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
    pub fn add_mock<Clock>(
        mut self,
        config: MockExecutionConfig,
        clock: Clock,
    ) -> Result<Self, BarterError>
    where
        Clock: EngineClock + Clone + Send + Sync + 'static,
    {
        const ACCOUNT_STREAM_CAPACITY: usize = 256;
        const DUMMY_EXECUTION_REQUEST_TIMEOUT: Duration = Duration::from_secs(1);

        let (request_tx, request_rx) = mpsc::unbounded_channel();
        let (event_tx, event_rx) = broadcast::channel(ACCOUNT_STREAM_CAPACITY);

        let mock_execution_client_config = MockExecutionClientConfig {
            mocked_exchange: config.mocked_exchange,
            clock: move || clock.time(),
            request_tx,
            event_rx,
        };

        // Register MockExchange init Future
        let mock_exchange_future = self.init_mock_exchange(config, request_rx, event_tx);
        self.mock_exchange_futures.push(mock_exchange_future);

        self.add_execution::<MockExecution<_>>(
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

        // Init ExecutionManager Future
        let future_result = ExecutionManager::init(
            execution_rx.into_stream(),
            request_timeout,
            Arc::new(Client::new(config)),
            AccountEventIndexer::new(Arc::new(instrument_map)),
            STREAM_RECONNECTION_POLICY,
        );

        let future_result = future_result.map(|result| {
            result.map(|(manager, account_stream)| {
                let manager_future: RunFuture = Box::pin(manager.run());
                let stream_future: RunFuture = Box::pin(account_stream.forward_to(merged_tx));

                (manager_future, stream_future)
            })
        });

        self.execution_init_futures.push(Box::pin(future_result));

        Ok(self)
    }

    /// Consume this `ExecutionBuilder` and build a full [`ExecutionBuild`] containing all the
    /// [`ExecutionManager`] (mock & live) and [`MockExchange`] futures.
    ///
    /// **For most users, calling [`ExecutionBuild::init`] after this is satisfactory.**
    ///
    /// If you want more control over what runtime drives the futures to completion, you can
    /// call [`ExecutionBuild::init_with_runtime`].
    pub fn build(mut self) -> ExecutionBuild {
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

        ExecutionBuild {
            execution_tx_map,
            account_channel: self.merged_channel,
            futures: ExecutionBuildFutures {
                mock_exchange_run_futures: self.mock_exchange_futures,
                execution_init_futures: self.execution_init_futures,
            },
        }
    }
}

/// Container holding execution infrastructure components ready to be initialised.
///
/// Call [`ExecutionBuild::init`] to run all the required execution component futures on tokio
/// tasks - returns the [`MultiExchangeTxMap`] and multi-exchange [`AccountStreamEvent`] stream.
#[allow(missing_debug_implementations)]
pub struct ExecutionBuild {
    pub execution_tx_map: MultiExchangeTxMap,
    pub account_channel: Channel<AccountStreamEvent>,
    pub futures: ExecutionBuildFutures,
}

impl ExecutionBuild {
    /// Initialises all execution components on the current tokio runtime.
    ///
    /// This method:
    /// - Spawns [`MockExchange`] runners tokio tasks.
    /// - Initialises all [`ExecutionManager`]s and their AccountStreams.
    /// - Returns the `MultiExchangeTxMap` and multi-exchange AccountStream.
    pub async fn init(self) -> Result<Execution, BarterError> {
        self.init_internal(tokio::runtime::Handle::current()).await
    }

    /// Initialises all execution components on the provided tokio runtime.
    ///
    /// Use this method if you want more control over which tokio runtime handles running
    /// execution components.
    ///
    /// This method:
    /// - Spawns [`MockExchange`] runners tokio tasks.
    /// - Initialises all [`ExecutionManager`]s and their AccountStreams.
    /// - Returns the `MultiExchangeTxMap` and multi-exchange AccountStream.
    pub async fn init_with_runtime(
        self,
        runtime: tokio::runtime::Handle,
    ) -> Result<Execution, BarterError> {
        self.init_internal(runtime).await
    }

    async fn init_internal(
        self,
        runtime: tokio::runtime::Handle,
    ) -> Result<Execution, BarterError> {
        let handles = self.futures.init_with_runtime(runtime).await?;

        Ok(Execution {
            execution_txs: self.execution_tx_map,
            account_channel: self.account_channel,
            handles,
        })
    }
}

#[allow(missing_debug_implementations)]
pub struct ExecutionBuildFutures {
    pub mock_exchange_run_futures: Vec<RunFuture>,
    pub execution_init_futures: Vec<ExecutionInitFuture>,
}

impl ExecutionBuildFutures {
    /// Initialises all execution components on the current tokio runtime.
    ///
    /// This method:
    /// - Spawns [`MockExchange`] runner tokio tasks.
    /// - Initialises all [`ExecutionManager`]s and their AccountStreams.
    /// - Spawns tokio tasks to forward AccountStreams to multi-exchange AccountStream
    pub async fn init(self) -> Result<ExecutionHandles, BarterError> {
        self.init_internal(tokio::runtime::Handle::current()).await
    }

    /// Initialises all execution components on the provided tokio runtime.
    ///
    /// Use this method if you want more control over which tokio runtime handles running
    /// execution components.
    ///
    /// This method:
    /// - Spawns [`MockExchange`] runner tokio tasks.
    /// - Initialises all [`ExecutionManager`]s and their AccountStreams.
    /// - Spawns tokio tasks to forward AccountStreams to multi-exchange AccountStream
    pub async fn init_with_runtime(
        self,
        runtime: tokio::runtime::Handle,
    ) -> Result<ExecutionHandles, BarterError> {
        self.init_internal(runtime).await
    }

    async fn init_internal(
        self,
        runtime: tokio::runtime::Handle,
    ) -> Result<ExecutionHandles, BarterError> {
        let mock_exchanges = self
            .mock_exchange_run_futures
            .into_iter()
            .map(|mock_exchange_run_future| runtime.spawn(mock_exchange_run_future))
            .collect();

        // Await ExecutionManager build futures and ensure success
        let (managers, account_to_engines) =
            futures::future::try_join_all(self.execution_init_futures)
                .await?
                .into_iter()
                .map(|(manager_run_future, account_event_forward_future)| {
                    (
                        runtime.spawn(manager_run_future),
                        runtime.spawn(account_event_forward_future),
                    )
                })
                .unzip();

        Ok(ExecutionHandles {
            mock_exchanges,
            managers,
            account_to_engines,
        })
    }
}

#[allow(missing_debug_implementations)]
pub struct ExecutionHandles {
    pub mock_exchanges: Vec<JoinHandle<()>>,
    pub managers: Vec<JoinHandle<()>>,
    pub account_to_engines: Vec<JoinHandle<()>>,
}

impl AsyncShutdown for ExecutionHandles {
    type Result = Result<(), JoinError>;

    async fn shutdown(&mut self) -> Self::Result {
        let handles = self
            .mock_exchanges
            .drain(..)
            .chain(self.managers.drain(..))
            .chain(self.account_to_engines.drain(..));

        try_join_all(handles).await?;
        Ok(())
    }
}

impl IntoIterator for ExecutionHandles {
    type Item = JoinHandle<()>;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.mock_exchanges
            .into_iter()
            .chain(self.managers)
            .chain(self.account_to_engines)
            .collect::<Vec<_>>()
            .into_iter()
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
