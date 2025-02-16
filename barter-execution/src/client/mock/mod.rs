use crate::{
    balance::AssetBalance,
    client::ExecutionClient,
    error::{UnindexedClientError, UnindexedOrderError},
    exchange::mock::request::MockExchangeRequest,
    order::{
        request::{OrderRequestCancel, OrderRequestOpen, UnindexedOrderResponseCancel},
        state::Open,
        Order, OrderEvent, OrderKey,
    },
    trade::Trade,
    UnindexedAccountEvent, UnindexedAccountSnapshot,
};
use barter_instrument::{
    asset::{name::AssetNameExchange, QuoteAsset},
    exchange::ExchangeId,
    instrument::name::InstrumentNameExchange,
};
use chrono::{DateTime, Utc};
use derive_more::Constructor;
use futures::stream::BoxStream;
use rust_decimal::Decimal;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio_stream::{wrappers::BroadcastStream, StreamExt};
use tracing::error;

#[derive(Debug, Clone, Constructor)]
pub struct MockExecutionConfig {
    pub mocked_exchange: ExchangeId,
    pub initial_state: UnindexedAccountSnapshot,
    pub latency_ms: u64,
    pub fees_percent: Decimal,
}

#[derive(Debug, Constructor)]
pub struct MockExecutionClientConfig {
    pub mocked_exchange: ExchangeId,
    pub request_tx: mpsc::UnboundedSender<MockExchangeRequest>,
    pub event_rx: broadcast::Receiver<UnindexedAccountEvent>,
}

impl Clone for MockExecutionClientConfig {
    fn clone(&self) -> Self {
        Self {
            mocked_exchange: self.mocked_exchange,
            request_tx: self.request_tx.clone(),
            event_rx: self.event_rx.resubscribe(),
        }
    }
}

#[derive(Debug, Constructor)]
pub struct MockExecution {
    pub mocked_exchange: ExchangeId,
    pub request_tx: mpsc::UnboundedSender<MockExchangeRequest>,
    pub event_rx: broadcast::Receiver<UnindexedAccountEvent>,
}

impl Clone for MockExecution {
    fn clone(&self) -> Self {
        Self {
            mocked_exchange: self.mocked_exchange,
            request_tx: self.request_tx.clone(),
            event_rx: self.event_rx.resubscribe(),
        }
    }
}

impl MockExecution {
    pub fn time_request(&self) -> DateTime<Utc> {
        // Todo: use input time_engine from requests once this is added
        Utc::now()
    }
}

impl ExecutionClient for MockExecution {
    const EXCHANGE: ExchangeId = ExchangeId::Mock;
    type Config = MockExecutionClientConfig;
    type AccountStream = BoxStream<'static, UnindexedAccountEvent>;

    fn new(config: Self::Config) -> Self {
        Self {
            mocked_exchange: config.mocked_exchange,
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
            .expect("MockExchange is offline - failed to send request");

        let snapshot = response_rx
            .await
            .expect("MockExchange if offline - failed to receive response");

        Ok(snapshot)
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
    ) -> UnindexedOrderResponseCancel {
        let (response_tx, response_rx) = oneshot::channel();

        self.request_tx
            .send(MockExchangeRequest::cancel_order(
                self.time_request(),
                response_tx,
                into_owned_request(request),
            ))
            .expect("MockExchange is offline - failed to send request");

        response_rx
            .await
            .expect("MockExchange if offline - failed to receive response")
    }

    async fn open_order(
        &self,
        request: OrderRequestOpen<ExchangeId, &InstrumentNameExchange>,
    ) -> Order<ExchangeId, InstrumentNameExchange, Result<Open, UnindexedOrderError>> {
        let (response_tx, response_rx) = oneshot::channel();

        self.request_tx
            .send(MockExchangeRequest::open_order(
                self.time_request(),
                response_tx,
                into_owned_request(request),
            ))
            .expect("MockExchange is offline - failed to send request");

        response_rx
            .await
            .expect("MockExchange if offline - failed to receive response")
    }

    async fn fetch_balances(
        &self,
    ) -> Result<Vec<AssetBalance<AssetNameExchange>>, UnindexedClientError> {
        let (response_tx, response_rx) = oneshot::channel();

        self.request_tx
            .send(MockExchangeRequest::fetch_balances(
                self.time_request(),
                response_tx,
            ))
            .expect("MockExchange is offline - failed to send request");

        let balances = response_rx
            .await
            .expect("MockExchange if offline - failed to receive response");

        Ok(balances)
    }

    async fn fetch_open_orders(
        &self,
    ) -> Result<Vec<Order<ExchangeId, InstrumentNameExchange, Open>>, UnindexedClientError> {
        let (response_tx, response_rx) = oneshot::channel();

        self.request_tx
            .send(MockExchangeRequest::fetch_orders_open(
                self.time_request(),
                response_tx,
            ))
            .expect("MockExchange is offline - failed to send request");

        let open_orders = response_rx
            .await
            .expect("MockExchange if offline - failed to receive response");

        Ok(open_orders)
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
            .expect("MockExchange is offline - failed to send request");

        let trades = response_rx
            .await
            .expect("MockExchange if offline - failed to receive response");

        Ok(trades)
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
