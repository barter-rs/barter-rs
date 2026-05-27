use crate::execution::{
    AccountStreamEvent,
    error::ExecutionError,
    request::{ExecutionRequest, RequestFuture},
};
use barter_data::streams::{
    consumer::StreamKey,
    reconnect::stream::{ReconnectingStream, ReconnectionBackoffPolicy, init_reconnecting_stream},
};
use barter_execution::{
    AccountEvent, AccountEventKind,
    client::ExecutionClient,
    error::{ConnectivityError, OrderError, UnindexedOrderError},
    indexer::{AccountEventIndexer, IndexedAccountStream},
    map::ExecutionInstrumentMap,
    order::{
        Order, UnindexedOrderSnapshot,
        request::{
            OrderRequestCancel, OrderRequestOpen, OrderResponseCancel, UnindexedOrderResponseCancel,
        },
        state::{FullyFilled, Open, OrderState},
    },
};
use barter_instrument::{
    asset::{AssetIndex, name::AssetNameExchange},
    exchange::{ExchangeId, ExchangeIndex},
    index::error::IndexError,
    instrument::{InstrumentIndex, name::InstrumentNameExchange},
};
use barter_integration::{
    channel::{Tx, UnboundedTx, mpsc_unbounded},
    collection::snapshot::Snapshot,
    stream::util::merge::merge,
};
use derive_more::Constructor;
use futures::{Stream, StreamExt, future::Either, stream::FuturesUnordered};
use std::sync::Arc;
use tracing::{error, info, warn};

/// Per-exchange execution manager that actions order requests from the Engine and forwards back
/// responses.
///
/// Processes indexed Engine [`ExecutionRequest`]s by:
/// - Transforming the requests to use the associated exchange's asset and instrument names.
/// - Issues the request via it's associated exchange [`ExecutionClient`],
/// - Tracks requests and returns timeouts to the Engine where necessary.
#[derive(Debug, Constructor)]
pub struct ExecutionManager<RequestStream, Client> {
    /// `Stream` of incoming Engine [`ExecutionRequest`]s.
    pub request_stream: RequestStream,

    /// Maximum `Duration` to wait for execution request responses from the [`ExecutionClient`].
    pub request_timeout: std::time::Duration,

    /// Transmitter for sending execution request responses back to the Engine.
    pub response_tx: UnboundedTx<AccountStreamEvent<ExchangeIndex, AssetIndex, InstrumentIndex>>,

    /// Exchange-specific [`ExecutionClient`] for executing orders.
    pub client: Arc<Client>,

    /// Mapper for converting between exchange-specific and index identifiers.
    ///
    /// For example, `InstrumentNameExchange` -> `InstrumentIndex`.
    pub indexer: AccountEventIndexer,
}

impl<RequestStream, Client> ExecutionManager<RequestStream, Client>
where
    RequestStream: Stream<Item = ExecutionRequest<ExchangeIndex, InstrumentIndex>> + Unpin,
    Client: ExecutionClient + Send + Sync,
    Client::AccountStream: Send,
{
    /// Initialises a new `ExecutionManager` and it's associated AccountStream.
    ///
    /// The first item of the AccountStream will be a full account snapshot.
    pub async fn init(
        request_stream: RequestStream,
        request_timeout: std::time::Duration,
        client: Arc<Client>,
        indexer: AccountEventIndexer,
        reconnect_policy: ReconnectionBackoffPolicy,
    ) -> Result<(Self, impl Stream<Item = AccountStreamEvent> + Send), ExecutionError> {
        // Determine StreamKey & ExchangeId for use in logging
        let stream_key = Self::determine_account_stream_key(&indexer.map)?;

        info!(
            exchange_index = %indexer.map.exchange.key,
            exchange_id = %indexer.map.exchange.value,
            policy = ?reconnect_policy,
            ?stream_key,
            "AccountStream with auto reconnect initialising"
        );

        // TODO: Would it be possible for us to fetch the snapshots of known orders after the reconnect?
        // Not requesting them from the engine. But initialize the fetch from here.
        // We could probably be subscribed to account_stream and react on the reconnect event.

        // Initialise reconnecting IndexedAccountStream (snapshot + updates)
        let client_clone = Arc::clone(&client);
        let indexer_clone = indexer.clone();
        let account_stream = init_reconnecting_stream(move || {
            let client = client_clone.clone();
            let indexer = indexer_clone.clone();
            async move {
                // Allocate AssetNameExchanges & InstrumentNameExchanges to avoid lifetime issues
                let assets = indexer.map.exchange_assets().cloned().collect::<Vec<_>>();
                let instruments = indexer
                    .map
                    .exchange_instruments()
                    .cloned()
                    .collect::<Vec<_>>();

                // Initialise AccountStream & apply indexing
                let updates = Self::init_indexed_account_stream(
                    &client,
                    indexer.clone(),
                    &assets,
                    &instruments,
                )
                .await?;

                // Fetch AccountSnapshot & index
                let snapshot =
                    Self::fetch_indexed_account_snapshot(&client, &indexer, &assets, &instruments)
                        .await?;

                // It's expected downstream consumers (eg/ EngineState will sync updates)
                Ok(futures::stream::once(std::future::ready(snapshot)).chain(updates))
            }
        })
        .await?;

        // Construct channel to communicate ExecutionRequest responses (ie/ AccountEvents) to Engine
        let (response_tx, response_rx) = mpsc_unbounded();

        // Construct merged IndexedAccountStream (execution responses + account notifications)
        let merged_account_stream = merge(
            response_rx.into_stream(),
            account_stream
                .with_reconnect_backoff::<_, ExecutionError>(reconnect_policy, stream_key)
                .with_reconnection_events(indexer.map.exchange.value),
        );

        Ok((
            Self::new(
                request_stream,
                request_timeout,
                response_tx,
                client,
                indexer,
            ),
            // TODO: Do not return the merged account stream. The ExecutionManager should sanitize
            // the events received before returning them to the subscriber
            merged_account_stream,
        ))
    }

    fn determine_account_stream_key(
        instrument_map: &Arc<ExecutionInstrumentMap>,
    ) -> Result<StreamKey, ExecutionError> {
        match (Client::EXCHANGE, instrument_map.exchange.value) {
            (ExchangeId::Mock, instrument_exchange) => Ok(StreamKey::new_general(
                "account_stream_mock",
                instrument_exchange,
            )),
            (ExchangeId::Simulated, instrument_exchange) => Ok(StreamKey::new_general(
                "account_stream_simulated",
                instrument_exchange,
            )),
            (client, instrument_exchange) if client == instrument_exchange => {
                Ok(StreamKey::new_general("account_stream", client))
            }
            (client, instrument_exchange) => Err(ExecutionError::Config(format!(
                "ExecutionManager Client ExchangeId: {client} does not match \
                    ExecutionInstrumentMap ExchangeId: {instrument_exchange}"
            ))),
        }
    }

    async fn fetch_indexed_account_snapshot(
        client: &Arc<Client>,
        indexer: &AccountEventIndexer,
        assets: &[AssetNameExchange],
        instruments: &[InstrumentNameExchange],
    ) -> Result<AccountEvent, ExecutionError> {
        match client.account_snapshot(assets, instruments).await {
            Ok(snapshot) => {
                let indexed_snapshot = indexer.snapshot(snapshot)?;
                Ok(AccountEvent {
                    exchange: indexer.map.exchange.key,
                    kind: AccountEventKind::Snapshot(indexed_snapshot),
                })
            }
            Err(error) => Err(ExecutionError::Client(indexer.client_error(error)?)),
        }
    }

    async fn init_indexed_account_stream(
        client: &Arc<Client>,
        indexer: AccountEventIndexer,
        assets: &[AssetNameExchange],
        instruments: &[InstrumentNameExchange],
    ) -> Result<impl Stream<Item = AccountEvent> + use<RequestStream, Client>, ExecutionError> {
        let stream = match client.account_stream(assets, instruments).await {
            Ok(stream) => stream,
            Err(error) => return Err(ExecutionError::Client(indexer.client_error(error)?)),
        };

        Ok(
            IndexedAccountStream::new(stream, indexer).filter_map(|result| {
                std::future::ready(match result {
                    Ok(indexed_event) => Some(indexed_event),
                    Err(error) => {
                        error!(
                            ?error,
                            "filtered IndexError produced by IndexedAccountStream"
                        );
                        None
                    }
                })
            }),
        )
    }

    /// Run the `ExecutionManager`, processing execution requests and forwarding back responses via
    /// the AccountStream.
    pub async fn run(mut self) {
        let mut in_flight_cancels = FuturesUnordered::new();
        let mut in_flight_opens = FuturesUnordered::new();
        let mut in_flight_snapshots = FuturesUnordered::new();

        loop {
            let next_cancel_response = if in_flight_cancels.is_empty() {
                Either::Left(std::future::pending())
            } else {
                Either::Right(in_flight_cancels.select_next_some())
            };

            let next_open_response = if in_flight_opens.is_empty() {
                Either::Left(std::future::pending())
            } else {
                Either::Right(in_flight_opens.select_next_some())
            };

            let next_snapshot_response = if in_flight_snapshots.is_empty() {
                Either::Left(std::future::pending())
            } else {
                Either::Right(in_flight_snapshots.select_next_some())
            };

            tokio::select! {
                // Process Engine ExecutionRequests
                request = self.request_stream.next() => match request {
                    Some(ExecutionRequest::Shutdown) | None => {
                        // TODO: The ExecutionManager should not shutdown until
                        // we processed all in flight requests.
                        break;
                    }
                    Some(ExecutionRequest::Cancel(request)) => {
                        // Panic since the system is set up incorrectly, so it's foolish to continue
                        let client_request = self
                            .indexer
                            .order_request(&request)
                            .unwrap_or_else(|error| panic!(
                                "ExecutionManager received cancel request for non-configured key: {error}"
                            ));

                        in_flight_cancels.push(RequestFuture::new(
                            self.client.cancel_order(client_request),
                            self.request_timeout,
                            request,
                        ))
                    },
                    Some(ExecutionRequest::Open(request)) => {
                        // Panic since the system is set up incorrectly, so it's foolish to continue
                        let client_request = self
                            .indexer
                            .order_request(&request)
                            .unwrap_or_else(|error| panic!(
                                "ExecutionManager received open request for non-configured key: {error}"
                            ));

                        in_flight_opens.push(RequestFuture::new(
                            self.client.open_order(client_request),
                            self.request_timeout,
                            request,
                        ))
                    }
                    Some(ExecutionRequest::Snapshots(requests)) => {
                        let client_requests = requests.iter().map(|request| {
                            // Panic since the system is set up incorrectly, so it's foolish to continue
                            self.indexer
                                .order_request(request)
                                .unwrap_or_else(|error| panic!(
                                    "ExecutionManager received snapshot request for non-configured key: {error}"
                                ))
                                .cloned_instrument()
                        }).collect::<Vec<_>>();

                        let futures = requests.into_iter()
                            .zip(self.client.fetch_order_snapshots(client_requests))
                            .map(|(request, response_fut)| RequestFuture::new(
                                response_fut,
                                self.request_timeout,
                                request,
                            ));

                        in_flight_snapshots.extend(futures);
                    }
                },

                // Process next ExecutionRequest::Cancel response
                response_cancel = next_cancel_response => {
                    let (request, result) = response_cancel;
                    match result {
                        Ok(Some(response)) => {
                            // TODO: The response here doesn't definitively prove that the order
                            // was cancelled. It's only an cancellation acknowledgment. The order can
                            // still be filled while in process of being cancelled.
                            let event = match self.process_cancel_response(response) {
                                Ok(indexed_event) => indexed_event,
                                Err(error) => {
                                    warn!(
                                        exchange = %self.indexer.map.exchange.value,
                                        ?error,
                                        "ExecutionManager filtering cancel response due to unrecognised index"
                                    );
                                    continue
                                }
                            };

                            if self.response_tx.send(event).is_err() {
                                break;
                            }
                        }
                        Err(_elapsed) => {
                            // TODO: We should try again. The timeout happens
                            // when we don't receive any response from the exchange.
                            // That can mean anything. It doesn't mean that the order
                            // was not cancelled.
                            let event = Self::process_cancel_timeout(request);

                            if self.response_tx.send(event).is_err() {
                                break;
                            }
                        }
                        Ok(None) => {
                            // TODO: Remove the Option return value. This was a temporary
                            // hack indicating that the response result shouldn't be used
                            // as an acknowledgment of the final status. This should be
                            // removed and solved differently.
                        }
                    };
                },

                // Process next ExecutionRequest::Open response
                response_open = next_open_response => {
                    let (request, result) = response_open;
                    match result {
                        Ok(Some(response)) => {
                            // TODO: The response here doesn't mean the order was actually opened. It's only an acknowledgment.
                            // We should listen to the events received from the account stream.
                            let event = match self.process_open_response(response) {
                                Ok(indexed_event) => indexed_event,
                                Err(error) => {
                                    warn!(
                                        exchange = %self.indexer.map.exchange.value,
                                        ?error,
                                        "ExecutionManager filtering open response due to unrecognised index"
                                    );
                                    continue
                                }
                            };

                            if self.response_tx.send(event).is_err() {
                                break;
                            }
                        }
                        Err(_elapsed) => {
                            // TODO: We should check if the order was opened or not. In some cases
                            // the request can timeout but the order was still opened. We should check
                            // the status of the order on the exchange before publishing the order event.
                            let event = Self::process_open_timeout(request);

                            if self.response_tx.send(event).is_err() {
                                break;
                            }
                        }
                        Ok(None) => {
                            // TODO: Remove the Option return value. This was a temporary
                            // hack indicating that the response result shouldn't be used
                            // as an acknowledgment of the final status. This should be
                            // solved differently.
                        }
                    }
                }

                // Process next ExecutionRequest::Snapshots response
                response_snapshot = next_snapshot_response => {
                    let (request, result) = response_snapshot;
                    match result {
                        Ok(Ok(response)) => {
                            // TODO: The current snapshot was received from the exchange.
                            // We should use this snapshot and check if the state on our side
                            // corresponds to the state on the exchange.
                            let event = match self.process_snapshot_response(response) {
                                Ok(indexed_event) => indexed_event,
                                Err(error) => {
                                    warn!(
                                        exchange = %self.indexer.map.exchange.value,
                                        ?error,
                                        "ExecutionManager filtering snapshot response due to unrecognised index"
                                    );
                                    continue
                                }
                            };

                            if self.response_tx.send(event).is_err() {
                                break;
                            }
                        }
                        Ok(Err(error)) => {
                            // TODO: It depends on the error. In some cases we should try again.
                            // In other we can say that the order does not exist.
                            warn!(
                                exchange = %self.indexer.map.exchange.value,
                                ?request,
                                ?error,
                                "ExecutionManager fetch_order_snapshot failed"
                            );
                        }
                        Err(_elapsed) => {
                            // TODO: We should always try again. The timeout happens
                            // when we don't receive any response from the exchange.
                            // That can mean anything. We should try again
                            warn!(
                                exchange = %self.indexer.map.exchange.value,
                                ?request,
                                "ExecutionManager fetch_order_snapshot timed out"
                            );
                        }
                    }
                }
            }
        }

        info!(
            exchange = %self.indexer.map.exchange.value,
            "ExecutionManager shutting down"
        )
    }

    fn process_cancel_response(
        &self,
        order: UnindexedOrderResponseCancel,
    ) -> Result<AccountStreamEvent, IndexError> {
        let order = self.indexer.order_response_cancel(order)?;

        Ok(AccountStreamEvent::Item(AccountEvent {
            exchange: order.key.exchange,
            kind: AccountEventKind::OrderCancelled(order),
        }))
    }

    fn process_cancel_timeout(
        order: OrderRequestCancel<ExchangeIndex, InstrumentIndex>,
    ) -> AccountStreamEvent {
        let OrderRequestCancel { key, state: _ } = order;

        AccountStreamEvent::Item(AccountEvent {
            exchange: key.exchange,
            kind: AccountEventKind::OrderCancelled(OrderResponseCancel {
                key,
                state: Err(OrderError::Connectivity(ConnectivityError::Timeout)),
            }),
        })
    }

    fn process_snapshot_response(
        &self,
        order: UnindexedOrderSnapshot,
    ) -> Result<AccountStreamEvent, IndexError> {
        let order = self.indexer.order_snapshot(order)?;

        Ok(AccountStreamEvent::Item(AccountEvent {
            exchange: order.key.exchange,
            kind: AccountEventKind::OrderSnapshot(Snapshot(order)),
        }))
    }

    fn process_open_response(
        &self,
        order: Order<ExchangeId, InstrumentNameExchange, Result<Open, UnindexedOrderError>>,
    ) -> Result<AccountStreamEvent, IndexError> {
        let Order {
            key,
            side,
            price,
            quantity,
            kind,
            time_in_force,
            state,
        } = order;

        let key = self.indexer.order_key(key)?;

        let state = match state {
            Ok(open) if open.quantity_remaining(quantity).is_zero() => {
                OrderState::FullyFilled(FullyFilled)
            }
            Ok(open) => OrderState::Open(open),
            Err(error) => OrderState::OpenFailed(self.indexer.order_error(error)?),
        };

        Ok(AccountStreamEvent::Item(AccountEvent {
            exchange: key.exchange,
            kind: AccountEventKind::OrderSnapshot(Snapshot(Order {
                key,
                side,
                price,
                quantity,
                kind,
                time_in_force,
                state,
            })),
        }))
    }

    fn process_open_timeout(
        order: OrderRequestOpen<ExchangeIndex, InstrumentIndex>,
    ) -> AccountStreamEvent {
        let OrderRequestOpen { key, state } = order;

        AccountStreamEvent::Item(AccountEvent {
            exchange: key.exchange,
            kind: AccountEventKind::OrderSnapshot(Snapshot(Order {
                key,
                side: state.side,
                price: state.price,
                quantity: state.quantity,
                kind: state.kind,
                time_in_force: state.time_in_force,
                state: OrderState::OpenFailed(OrderError::Connectivity(ConnectivityError::Timeout)),
            })),
        })
    }
}
