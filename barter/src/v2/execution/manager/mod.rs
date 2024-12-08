use crate::v2::{
    balance::AssetBalance,
    error::{IndexError, KeyError},
    execution::{
        error::{
            ConnectivityError, ExecutionError, IndexedApiError, IndexedClientError,
            UnindexedApiError, UnindexedClientError,
        },
        manager::client::ExecutionClient,
        map::ExecutionInstrumentMap,
        AccountEvent, AccountEventKind, ExecutionRequest, IndexedAccountEvent,
        IndexedAccountSnapshot, InstrumentAccountSnapshot, UnindexedAccountEvent,
        UnindexedAccountSnapshot,
    },
    order::{Cancelled, ExchangeOrderState, Open, Order, RequestCancel, RequestOpen},
    position::PositionExchange,
    trade::{AssetFees, Trade},
    Snapshot,
};
use barter_data::streams::{
    consumer::StreamKey,
    reconnect,
    reconnect::stream::{init_reconnecting_stream, ReconnectingStream, ReconnectionBackoffPolicy},
};
use barter_instrument::{
    asset::{name::AssetNameExchange, AssetIndex},
    exchange::{ExchangeId, ExchangeIndex},
    instrument::{name::InstrumentNameExchange, InstrumentIndex},
};
use barter_integration::{
    channel::{mpsc_unbounded, Tx, UnboundedTx},
    stream::merge::merge,
};
use derive_more::Constructor;
use futures::{future::Either, stream::FuturesUnordered, Stream, StreamExt};
use pin_project::pin_project;
use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tracing::{error, info, warn};

pub mod client;

pub type AccountStreamEvent<ExchangeKey, AssetKey, InstrumentKey> =
    reconnect::Event<ExchangeId, AccountEvent<ExchangeKey, AssetKey, InstrumentKey>>;

pub type IndexedAccountStreamEvent = AccountStreamEvent<ExchangeIndex, AssetIndex, InstrumentIndex>;

pub type ExecutionInitResult<Manager, AccountStream> =
    Result<(Manager, AccountStream), ExecutionError>;

/// Per exchange execution manager.
#[derive(Debug, Constructor)]
pub struct ExecutionManager<RequestStream, Client> {
    pub request_stream: RequestStream,
    pub request_timeout: std::time::Duration,
    pub response_tx: UnboundedTx<AccountStreamEvent<ExchangeIndex, AssetIndex, InstrumentIndex>>,
    pub client: Arc<Client>,
    pub indexer: AccountEventIndexer,
}

impl<RequestStream, Client> ExecutionManager<RequestStream, Client>
where
    RequestStream: Stream<Item = ExecutionRequest<ExchangeIndex, InstrumentIndex>> + Unpin,
    Client: ExecutionClient + Send + Sync,
    Client::AccountStream: Send,
{
    pub async fn init(
        request_stream: RequestStream,
        request_timeout: std::time::Duration,
        client: Arc<Client>,
        indexer: AccountEventIndexer,
        reconnect_policy: ReconnectionBackoffPolicy,
    ) -> Result<(Self, impl Stream<Item = IndexedAccountStreamEvent> + Send), ExecutionError> {
        // Determine StreamKey & ExchangeId for use in logging
        let stream_key = Self::determine_account_stream_key(&indexer.map)?;

        info!(
            exchange_index = %indexer.map.exchange.key,
            exchange_id = %indexer.map.exchange.value,
            policy = ?reconnect_policy,
            ?stream_key,
            "AccountStream with auto reconnect initialising"
        );

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
    ) -> Result<IndexedAccountEvent, ExecutionError> {
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
    ) -> Result<impl Stream<Item = IndexedAccountEvent>, ExecutionError> {
        let stream = match client.account_stream(assets, instruments).await {
            Ok(stream) => stream,
            Err(error) => return Err(ExecutionError::Client(indexer.client_error(error)?)),
        };

        Ok(
            IndexedAccountStream::new(indexer, stream).filter_map(|result| {
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

    async fn init_synchronised_account_snapshot_with_updates() {}

    // async fn sync_account_snapshot_and_updates(
    //     snapshot: IndexedAccountSnapshot,
    //     updates: impl Stream<Item = IndexedAccountEvent>
    // ) -> impl Stream<Item = IndexedAccountEvent>
    // {
    //     updates
    //         .scan(snapshot, |snapshot, event| {
    //
    //             // Todo:
    //
    //             match event.kind {
    //                 AccountEventKind::Snapshot(snapshot) => {
    //
    //                 }
    //                 AccountEventKind::BalanceSnapshot(Snapshot(event)) => {
    //                     if let Some(snapshot) = snapshot
    //                         .balances
    //                         .iter_mut()
    //                         .find(|snapshot| {
    //                             snapshot.asset == event.asset
    //                         })
    //                     {
    //                         if snapshot.time_exchange <= event.time_exchange {
    //                             snapshot.balance = event.balance;
    //                         }
    //                     } else {
    //                         warn!(
    //                             ?snapshot,
    //                             ?event,
    //                             "AccountSnapshot encountered Balance for non-tracked Asset - adding"
    //                         );
    //                         snapshot.balances.push(event);
    //                     }
    //                 }
    //                 AccountEventKind::PositionSnapshot(Snapshot(position_)) => {
    //
    //                 }
    //                 AccountEventKind::OrderSnapshot(_) => {}
    //                 AccountEventKind::OrderOpened(_) => {}
    //                 AccountEventKind::OrderCancelled(_) => {}
    //                 AccountEventKind::Trade(_) => {}
    //             }
    //
    //
    //         })
    // }

    pub async fn run(mut self) {
        let mut in_flight_cancels = FuturesUnordered::new();
        let mut in_flight_opens = FuturesUnordered::new();

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

            tokio::select! {
                // Process Engine ExecutionRequests
                request = self.request_stream.next() => match request {
                    Some(ExecutionRequest::Cancel(request)) => {
                        // Todo: remove need for Cloning
                        let request_clone = request.clone();

                        // Panic since the system is set up incorrectly, so it's foolish to continue
                        let request = self
                            .indexer
                            .order_request(request)
                            .unwrap_or_else(|error| panic!(
                                "ExecutionManager received cancel request for non-configured key: {error}"
                            ));

                        in_flight_cancels.push(RequestFuture::new(
                            request_clone,
                            self.request_timeout,
                            self.client.cancel_order(request)
                        ))
                    },
                    Some(ExecutionRequest::Open(request)) => {
                        // Todo: remove need for Cloning
                        let request_clone = request.clone();

                        // Panic since the system is set up incorrectly, so it's foolish to continue
                        let request = self
                            .indexer
                            .order_request(request)
                            .unwrap_or_else(|error| panic!(
                                "ExecutionManager received open request for non-configured key: {error}"
                            ));

                        in_flight_opens.push(RequestFuture::new(
                            request_clone,
                            self.request_timeout,
                            self.client.open_order(request)
                        ))
                    }
                    None => {
                        return
                    },
                },

                // Process next ExecutionRequest::Cancel response
                response_cancel = next_cancel_response => {
                    let event = match response_cancel {
                        Ok(response) => {
                            match self.process_cancel_response(response) {
                                Ok(indexed_event) => indexed_event,
                                Err(error) => {
                                    warn!(
                                        exchange = ?self.indexer.map.exchange,
                                        ?error,
                                        "ExecutionManager filtering cancel response due to unrecognised index"
                                    );
                                    continue
                                }
                            }
                        }
                        Err(request) => {
                            self.process_cancel_timeout(request)
                        }
                    };

                    if self.response_tx.send(event).is_err() {
                        break;
                    }
                },

                // Process next ExecutionRequest::Open response
                response_open = next_open_response => {
                    let event = match response_open {
                        Ok(response) => {
                            match self.process_open_response(response) {
                                Ok(indexed_event) => indexed_event,
                                Err(error) => {
                                    warn!(
                                        exchange = ?self.indexer.map.exchange,
                                        ?error,
                                        "ExecutionManager filtering open response due to unrecognised index"
                                    );
                                    continue
                                }
                            }
                        }
                        Err(request) => {
                            self.process_open_timeout(request)
                        }
                    };

                    if self.response_tx.send(event).is_err() {
                        break;
                    }
                }

            }
        }
    }

    pub fn process_cancel_response(
        &self,
        order: Order<ExchangeId, InstrumentNameExchange, Result<Cancelled, UnindexedClientError>>,
    ) -> Result<IndexedAccountStreamEvent, IndexError> {
        let order = self.indexer.order_response(order)?;

        Ok(IndexedAccountStreamEvent::Item(AccountEvent {
            exchange: order.exchange,
            kind: AccountEventKind::OrderCancelled(order),
        }))
    }

    pub fn process_cancel_timeout(
        &self,
        order: Order<ExchangeIndex, InstrumentIndex, RequestCancel>,
    ) -> IndexedAccountStreamEvent {
        let Order {
            exchange,
            instrument,
            cid,
            side,
            state: _,
        } = order;

        IndexedAccountStreamEvent::Item(AccountEvent {
            exchange,
            kind: AccountEventKind::OrderCancelled(Order {
                exchange,
                instrument,
                cid,
                side,
                state: Err(IndexedClientError::Connectivity(ConnectivityError::Timeout)),
            }),
        })
    }

    pub fn process_open_response(
        &self,
        order: Order<ExchangeId, InstrumentNameExchange, Result<Open, UnindexedClientError>>,
    ) -> Result<IndexedAccountStreamEvent, IndexError> {
        let order = self.indexer.order_response(order)?;

        Ok(IndexedAccountStreamEvent::Item(AccountEvent {
            exchange: order.exchange,
            kind: AccountEventKind::OrderOpened(order),
        }))
    }

    pub fn process_open_timeout(
        &self,
        order: Order<ExchangeIndex, InstrumentIndex, RequestOpen>,
    ) -> IndexedAccountStreamEvent {
        let Order {
            exchange,
            instrument,
            cid,
            side,
            state: _,
        } = order;

        IndexedAccountStreamEvent::Item(AccountEvent {
            exchange,
            kind: AccountEventKind::OrderOpened(Order {
                exchange,
                instrument,
                cid,
                side,
                state: Err(IndexedClientError::Connectivity(ConnectivityError::Timeout)),
            }),
        })
    }
}
#[derive(Debug)]
#[pin_project::pin_project]
pub struct RequestFuture<Request, ResponseFut> {
    request: Request,
    #[pin]
    response_future: tokio::time::Timeout<ResponseFut>,
}

impl<Request, ResponseFut> Future for RequestFuture<Request, ResponseFut>
where
    Request: Clone,
    ResponseFut: Future,
{
    type Output = Result<ResponseFut::Output, Request>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        this.response_future
            .poll(cx)
            .map(|result| result.map_err(|_| this.request.clone()))
    }
}

impl<Request, ResponseFut> RequestFuture<Request, ResponseFut>
where
    ResponseFut: Future,
{
    pub fn new(request: Request, timeout: std::time::Duration, future: ResponseFut) -> Self {
        Self {
            request,
            response_future: tokio::time::timeout(timeout, future),
        }
    }
}

pub type IndexedAccountStream<St> = IndexedStream<AccountEventIndexer, St>;

#[derive(Debug, Constructor)]
#[pin_project]
pub struct IndexedStream<Indexer, Stream> {
    pub indexer: Indexer,
    #[pin]
    pub stream: Stream,
}

pub trait Indexer {
    type Unindexed;
    type Indexed;
    fn index(&self, item: Self::Unindexed) -> Result<Self::Indexed, IndexError>;
}

impl<Index, St> Stream for IndexedStream<Index, St>
where
    Index: Indexer<Unindexed = St::Item>,
    St: Stream,
{
    type Item = Result<Index::Indexed, IndexError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        match this.stream.poll_next(cx) {
            Poll::Ready(Some(item)) => Poll::Ready(Some(this.indexer.index(item))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[derive(Debug, Clone, Constructor)]
pub struct AccountEventIndexer {
    pub map: Arc<ExecutionInstrumentMap>,
}

impl Indexer for AccountEventIndexer {
    type Unindexed = UnindexedAccountEvent;
    type Indexed = IndexedAccountEvent;

    fn index(&self, item: Self::Unindexed) -> Result<Self::Indexed, IndexError> {
        self.account_event(item)
    }
}

impl AccountEventIndexer {
    pub fn account_event(
        &self,
        event: UnindexedAccountEvent,
    ) -> Result<IndexedAccountEvent, IndexError> {
        let UnindexedAccountEvent { exchange, kind } = event;

        let exchange = self.map.find_exchange_index(exchange)?;

        let kind = match kind {
            AccountEventKind::Snapshot(snapshot) => {
                AccountEventKind::Snapshot(self.snapshot(snapshot)?)
            }
            AccountEventKind::BalanceSnapshot(snapshot) => {
                AccountEventKind::BalanceSnapshot(self.asset_balance(snapshot.0).map(Snapshot)?)
            }
            AccountEventKind::PositionSnapshot(snapshot) => {
                AccountEventKind::PositionSnapshot(self.position(snapshot.0).map(Snapshot)?)
            }
            AccountEventKind::OrderSnapshot(snapshot) => {
                AccountEventKind::OrderSnapshot(self.order_snapshot(snapshot.0).map(Snapshot)?)
            }
            AccountEventKind::OrderOpened(order) => {
                AccountEventKind::OrderOpened(self.order_response(order)?)
            }
            AccountEventKind::OrderCancelled(order) => {
                AccountEventKind::OrderCancelled(self.order_response(order)?)
            }
            AccountEventKind::Trade(trade) => AccountEventKind::Trade(self.trade(trade)?),
        };

        Ok(IndexedAccountEvent { exchange, kind })
    }

    pub fn snapshot(
        &self,
        snapshot: UnindexedAccountSnapshot,
    ) -> Result<IndexedAccountSnapshot, IndexError> {
        let UnindexedAccountSnapshot {
            balances,
            instruments,
        } = snapshot;

        let balances = balances
            .into_iter()
            .map(|balance| self.asset_balance(balance))
            .collect::<Result<Vec<_>, _>>()?;

        let instruments = instruments
            .into_iter()
            .map(|snapshot| {
                let InstrumentAccountSnapshot { position, orders } = snapshot;

                let position = self.position(position)?;
                let orders = orders
                    .into_iter()
                    .map(|order| self.order_open(order))
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(InstrumentAccountSnapshot { position, orders })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(IndexedAccountSnapshot {
            balances,
            instruments,
        })
    }

    pub fn asset_balance(
        &self,
        balance: AssetBalance<AssetNameExchange>,
    ) -> Result<AssetBalance<AssetIndex>, IndexError> {
        let AssetBalance {
            asset,
            balance,
            time_exchange,
        } = balance;
        let asset = self.map.find_asset_index(&asset)?;

        Ok(AssetBalance {
            asset,
            balance,
            time_exchange,
        })
    }

    pub fn position(
        &self,
        position: PositionExchange<InstrumentNameExchange>,
    ) -> Result<PositionExchange<InstrumentIndex>, IndexError> {
        let PositionExchange {
            instrument,
            side,
            price_entry_average,
            quantity_abs,
            time_exchange_update,
        } = position;

        let instrument = self.map.find_instrument_index(&instrument)?;

        Ok(PositionExchange {
            instrument,
            side,
            price_entry_average,
            quantity_abs,
            time_exchange_update,
        })
    }

    pub fn order_snapshot(
        &self,
        order: Order<ExchangeId, InstrumentNameExchange, ExchangeOrderState>,
    ) -> Result<Order<ExchangeIndex, InstrumentIndex, ExchangeOrderState>, IndexError> {
        let Order {
            exchange,
            instrument,
            cid,
            side,
            state,
        } = order;

        Ok(Order {
            exchange: self.map.find_exchange_index(exchange)?,
            instrument: self.map.find_instrument_index(&instrument)?,
            cid,
            side,
            state,
        })
    }

    pub fn order_request<Kind>(
        &self,
        order: Order<ExchangeIndex, InstrumentIndex, Kind>,
    ) -> Result<Order<ExchangeId, &InstrumentNameExchange, Kind>, KeyError> {
        let Order {
            exchange,
            instrument,
            cid,
            side,
            state,
        } = order;

        let exchange = self.map.find_exchange_id(exchange)?;
        let instrument = self.map.find_instrument_name_exchange(instrument)?;

        Ok(Order {
            exchange,
            instrument,
            cid,
            side,
            state,
        })
    }

    pub fn order_open(
        &self,
        order: Order<ExchangeId, InstrumentNameExchange, ExchangeOrderState>,
    ) -> Result<Order<ExchangeIndex, InstrumentIndex, ExchangeOrderState>, IndexError> {
        let Order {
            exchange,
            instrument,
            cid,
            side,
            state,
        } = order;

        Ok(Order {
            exchange: self.map.find_exchange_index(exchange)?,
            instrument: self.map.find_instrument_index(&instrument)?,
            cid,
            side,
            state,
        })
    }

    pub fn order_response<Kind>(
        &self,
        order: Order<ExchangeId, InstrumentNameExchange, Result<Kind, UnindexedClientError>>,
    ) -> Result<Order<ExchangeIndex, InstrumentIndex, Result<Kind, IndexedClientError>>, IndexError>
    {
        let Order {
            exchange,
            instrument,
            cid,
            side,
            state,
        } = order;

        let exchange_index = self.map.find_exchange_index(exchange)?;
        let instrument_index = self.map.find_instrument_index(&instrument)?;

        let state = match state {
            Ok(state) => Ok(state),
            Err(error) => Err(self.client_error(error)?),
        };

        Ok(Order {
            exchange: exchange_index,
            instrument: instrument_index,
            cid,
            side,
            state,
        })
    }

    pub fn client_error(
        &self,
        error: UnindexedClientError,
    ) -> Result<IndexedClientError, IndexError> {
        Ok(match error {
            UnindexedClientError::Connectivity(error) => IndexedClientError::Connectivity(error),
            UnindexedClientError::Api(error) => IndexedClientError::Api(match error {
                UnindexedApiError::RateLimit => IndexedApiError::RateLimit,
                UnindexedApiError::AssetInvalid(asset, value) => {
                    IndexedApiError::AssetInvalid(self.map.find_asset_index(&asset)?, value)
                }
                UnindexedApiError::InstrumentInvalid(instrument, value) => {
                    IndexedApiError::InstrumentInvalid(
                        self.map.find_instrument_index(&instrument)?,
                        value,
                    )
                }
                UnindexedApiError::BalanceInsufficient(asset, value) => {
                    IndexedApiError::BalanceInsufficient(self.map.find_asset_index(&asset)?, value)
                }
                UnindexedApiError::OrderRejected(cid) => IndexedApiError::OrderRejected(cid),
                UnindexedApiError::OrderAlreadyCancelled(cid) => {
                    IndexedApiError::OrderRejected(cid)
                }
                UnindexedApiError::OrderAlreadyFullyFilled(cid) => {
                    IndexedApiError::OrderRejected(cid)
                }
            }),
            UnindexedClientError::AccountSnapshot(value) => {
                IndexedClientError::AccountSnapshot(value)
            }
            UnindexedClientError::AccountStream(value) => IndexedClientError::AccountStream(value),
        })
    }

    pub fn trade(
        &self,
        trade: Trade<AssetNameExchange, InstrumentNameExchange>,
    ) -> Result<Trade<AssetIndex, InstrumentIndex>, IndexError> {
        let Trade {
            id,
            instrument,
            order_id,
            time_exchange,
            side,
            price,
            quantity,
            fees: AssetFees { asset, fees },
        } = trade;

        let instrument_index = self.map.find_instrument_index(&instrument)?;

        let asset_index = if let Some(asset) = asset {
            Some(self.map.find_asset_index(&asset)?)
        } else {
            None
        };

        Ok(Trade {
            id,
            instrument: instrument_index,
            order_id,
            time_exchange,
            side,
            price,
            quantity,
            fees: AssetFees {
                asset: asset_index,
                fees,
            },
        })
    }
}
