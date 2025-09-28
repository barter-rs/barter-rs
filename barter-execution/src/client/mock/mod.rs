use crate::{
    UnindexedAccountEvent, UnindexedAccountSnapshot,
    balance::AssetBalance,
    client::ExecutionClient,
    error::{ConnectivityError, UnindexedClientError, UnindexedOrderError},
    exchange::mock::request::MockExchangeRequest,
    order::{
        Order, OrderEvent, OrderKey,
        request::{OrderRequestCancel, OrderRequestOpen, UnindexedOrderResponseCancel},
        state::Open,
    },
    trade::Trade,
};
use barter_instrument::{
    asset::{QuoteAsset, name::AssetNameExchange},
    exchange::ExchangeId,
    instrument::name::InstrumentNameExchange,
};
use chrono::{DateTime, Utc};
use derive_more::Constructor;
use futures::stream::BoxStream;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio_stream::{StreamExt, wrappers::BroadcastStream};
use tracing::error;

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Constructor,
)]
pub struct MockExecutionConfig {
    pub mocked_exchange: ExchangeId,
    pub initial_state: UnindexedAccountSnapshot,
    pub latency_ms: u64,
    pub fees_percent: Decimal,
}

#[derive(Debug, Constructor)]
pub struct MockExecutionClientConfig<FnTime> {
    pub mocked_exchange: ExchangeId,
    pub clock: FnTime,
    pub request_tx: mpsc::UnboundedSender<MockExchangeRequest>,
    pub event_rx: broadcast::Receiver<UnindexedAccountEvent>,
}

impl<FnTime> Clone for MockExecutionClientConfig<FnTime>
where
    FnTime: Clone,
{
    fn clone(&self) -> Self {
        Self {
            mocked_exchange: self.mocked_exchange,
            clock: self.clock.clone(),
            request_tx: self.request_tx.clone(),
            event_rx: self.event_rx.resubscribe(),
        }
    }
}

#[derive(Debug, Constructor)]
pub struct MockExecution<FnTime> {
    pub mocked_exchange: ExchangeId,
    pub clock: FnTime,
    pub request_tx: mpsc::UnboundedSender<MockExchangeRequest>,
    pub event_rx: broadcast::Receiver<UnindexedAccountEvent>,
}

impl<FnTime> Clone for MockExecution<FnTime>
where
    FnTime: Clone,
{
    fn clone(&self) -> Self {
        Self {
            mocked_exchange: self.mocked_exchange,
            clock: self.clock.clone(),
            request_tx: self.request_tx.clone(),
            event_rx: self.event_rx.resubscribe(),
        }
    }
}

impl<FnTime> MockExecution<FnTime>
where
    FnTime: Fn() -> DateTime<Utc>,
{
    pub fn time_request(&self) -> DateTime<Utc> {
        (self.clock)()
    }
}

impl<FnTime> ExecutionClient for MockExecution<FnTime>
where
    FnTime: Fn() -> DateTime<Utc> + Clone + Sync,
{
    const EXCHANGE: ExchangeId = ExchangeId::Mock;
    type Config = MockExecutionClientConfig<FnTime>;
    type AccountStream = BoxStream<'static, UnindexedAccountEvent>;

    fn new(config: Self::Config) -> Self {
        Self {
            mocked_exchange: config.mocked_exchange,
            clock: config.clock,
            request_tx: config.request_tx,
            event_rx: config.event_rx,
        }
    }

    async fn account_snapshot(
        &self,
        _: &[AssetNameExchange],
        _: &[InstrumentNameExchange],
    ) -> Result<UnindexedAccountSnapshot, UnindexedClientError> {
        let (response_tx, response_rx) = oneshot::channel();

        self.request_tx
            .send(MockExchangeRequest::fetch_account_snapshot(
                self.time_request(),
                response_tx,
            ))
            .map_err(|_| {
                UnindexedClientError::Connectivity(ConnectivityError::ExchangeOffline(
                    self.mocked_exchange,
                ))
            })?;

        response_rx.await.map_err(|_| {
            UnindexedClientError::Connectivity(ConnectivityError::ExchangeOffline(
                self.mocked_exchange,
            ))
        })
    }

    async fn account_stream(
        &self,
        _: &[AssetNameExchange],
        _: &[InstrumentNameExchange],
    ) -> Result<Self::AccountStream, UnindexedClientError> {
        Ok(futures::StreamExt::boxed(
            BroadcastStream::new(self.event_rx.resubscribe()).map_while(|result| match result {
                Ok(event) => Some(event),
                Err(error) => {
                    error!(
                        ?error,
                        "MockExchange Broadcast AccountStream lagged - terminating"
                    );
                    None
                }
            }),
        ))
    }

    async fn cancel_order(
        &self,
        request: OrderRequestCancel<ExchangeId, &InstrumentNameExchange>,
    ) -> Option<UnindexedOrderResponseCancel> {
        let (response_tx, response_rx) = oneshot::channel();

        let key = OrderKey {
            exchange: request.key.exchange,
            instrument: request.key.instrument.clone(),
            strategy: request.key.strategy.clone(),
            cid: request.key.cid.clone(),
        };

        if self
            .request_tx
            .send(MockExchangeRequest::cancel_order(
                self.time_request(),
                response_tx,
                into_owned_request(request),
            ))
            .is_err()
        {
            return Some(UnindexedOrderResponseCancel {
                key,
                state: Err(UnindexedOrderError::Connectivity(
                    ConnectivityError::ExchangeOffline(self.mocked_exchange),
                )),
            });
        }

        Some(match response_rx.await {
            Ok(response) => response,
            Err(_) => UnindexedOrderResponseCancel {
                key,
                state: Err(UnindexedOrderError::Connectivity(
                    ConnectivityError::ExchangeOffline(self.mocked_exchange),
                )),
            },
        })
    }

    async fn open_order(
        &self,
        request: OrderRequestOpen<ExchangeId, &InstrumentNameExchange>,
    ) -> Option<Order<ExchangeId, InstrumentNameExchange, Result<Open, UnindexedOrderError>>> {
        let (response_tx, response_rx) = oneshot::channel();

        let request = into_owned_request(request);

        if self
            .request_tx
            .send(MockExchangeRequest::open_order(
                self.time_request(),
                response_tx,
                request.clone(),
            ))
            .is_err()
        {
            return Some(Order {
                key: request.key,
                side: request.state.side,
                price: request.state.price,
                quantity: request.state.quantity,
                kind: request.state.kind,
                time_in_force: request.state.time_in_force,
                state: Err(UnindexedOrderError::Connectivity(
                    ConnectivityError::ExchangeOffline(self.mocked_exchange),
                )),
            });
        }

        Some(match response_rx.await {
            Ok(response) => response,
            Err(_) => Order {
                key: request.key,
                side: request.state.side,
                price: request.state.price,
                quantity: request.state.quantity,
                kind: request.state.kind,
                time_in_force: request.state.time_in_force,
                state: Err(UnindexedOrderError::Connectivity(
                    ConnectivityError::ExchangeOffline(self.mocked_exchange),
                )),
            },
        })
    }

    async fn fetch_balances(
        &self,
        assets: &[AssetNameExchange],
    ) -> Result<Vec<AssetBalance<AssetNameExchange>>, UnindexedClientError> {
        let (response_tx, response_rx) = oneshot::channel();

        self.request_tx
            .send(MockExchangeRequest::fetch_balances(
                self.time_request(),
                assets.to_vec(),
                response_tx,
            ))
            .map_err(|_| {
                UnindexedClientError::Connectivity(ConnectivityError::ExchangeOffline(
                    self.mocked_exchange,
                ))
            })?;

        response_rx.await.map_err(|_| {
            UnindexedClientError::Connectivity(ConnectivityError::ExchangeOffline(
                self.mocked_exchange,
            ))
        })
    }

    async fn fetch_open_orders(
        &self,
        instruments: &[InstrumentNameExchange],
    ) -> Result<Vec<Order<ExchangeId, InstrumentNameExchange, Open>>, UnindexedClientError> {
        let (response_tx, response_rx) = oneshot::channel();

        self.request_tx
            .send(MockExchangeRequest::fetch_orders_open(
                self.time_request(),
                instruments.to_vec(),
                response_tx,
            ))
            .map_err(|_| {
                UnindexedClientError::Connectivity(ConnectivityError::ExchangeOffline(
                    self.mocked_exchange,
                ))
            })?;

        response_rx.await.map_err(|_| {
            UnindexedClientError::Connectivity(ConnectivityError::ExchangeOffline(
                self.mocked_exchange,
            ))
        })
    }

    async fn fetch_trades(
        &self,
        time_since: DateTime<Utc>,
    ) -> Result<Vec<Trade<QuoteAsset, InstrumentNameExchange>>, UnindexedClientError> {
        let (response_tx, response_rx) = oneshot::channel();

        self.request_tx
            .send(MockExchangeRequest::fetch_trades(
                self.time_request(),
                response_tx,
                time_since,
            ))
            .map_err(|_| {
                UnindexedClientError::Connectivity(ConnectivityError::ExchangeOffline(
                    self.mocked_exchange,
                ))
            })?;

        response_rx.await.map_err(|_| {
            UnindexedClientError::Connectivity(ConnectivityError::ExchangeOffline(
                self.mocked_exchange,
            ))
        })
    }
}

fn into_owned_request<Kind>(
    request: OrderEvent<Kind, ExchangeId, &InstrumentNameExchange>,
) -> OrderEvent<Kind, ExchangeId, InstrumentNameExchange> {
    let OrderEvent {
        key:
            OrderKey {
                exchange,
                instrument,
                strategy,
                cid,
            },
        state,
    } = request;

    OrderEvent {
        key: OrderKey {
            exchange,
            instrument: instrument.clone(),
            strategy,
            cid,
        },
        state,
    }
}
