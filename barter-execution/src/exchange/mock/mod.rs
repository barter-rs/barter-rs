use crate::{
    AccountEventKind, InstrumentAccountSnapshot, UnindexedAccountEvent, UnindexedAccountSnapshot,
    balance::AssetBalance,
    client::mock::MockExecutionConfig,
    error::{ApiError, UnindexedApiError, UnindexedOrderError},
    exchange::mock::{
        account::AccountState,
        request::{MockExchangeRequest, MockExchangeRequestKind},
    },
    order::{
        Order, OrderKind, UnindexedOrder,
        id::OrderId,
        request::{OrderRequestCancel, OrderRequestOpen},
        state::{Cancelled, Open},
    },
    trade::{AssetFees, Trade, TradeId},
};
use barter_instrument::{
    Side,
    asset::{QuoteAsset, name::AssetNameExchange},
    exchange::ExchangeId,
    instrument::{Instrument, name::InstrumentNameExchange},
};
use barter_integration::snapshot::Snapshot;
use chrono::{DateTime, TimeDelta, Utc};
use fnv::FnvHashMap;
use futures::stream::BoxStream;
use itertools::Itertools;
use rust_decimal::Decimal;
use smol_str::ToSmolStr;
use std::fmt::Debug;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio_stream::{StreamExt, wrappers::BroadcastStream};
use tracing::{error, info};

pub mod account;
pub mod request;

#[derive(Debug)]
pub struct MockExchange {
    pub exchange: ExchangeId,
    pub latency_ms: u64,
    pub fees_percent: Decimal,
    pub request_rx: mpsc::UnboundedReceiver<MockExchangeRequest>,
    pub event_tx: broadcast::Sender<UnindexedAccountEvent>,
    pub instruments: FnvHashMap<InstrumentNameExchange, Instrument<ExchangeId, AssetNameExchange>>,
    pub account: AccountState,
    pub order_sequence: u64,
    pub time_exchange_latest: DateTime<Utc>,
}

impl MockExchange {
    pub fn new(
        config: MockExecutionConfig,
        request_rx: mpsc::UnboundedReceiver<MockExchangeRequest>,
        event_tx: broadcast::Sender<UnindexedAccountEvent>,
        instruments: FnvHashMap<InstrumentNameExchange, Instrument<ExchangeId, AssetNameExchange>>,
    ) -> Self {
        Self {
            exchange: config.mocked_exchange,
            latency_ms: config.latency_ms,
            fees_percent: config.fees_percent,
            request_rx,
            event_tx,
            instruments,
            account: AccountState::from(config.initial_state),
            order_sequence: 0,
            time_exchange_latest: Default::default(),
        }
    }

    pub async fn run(mut self) {
        while let Some(request) = self.request_rx.recv().await {
            self.update_time_exchange(request.time_request);

            match request.kind {
                MockExchangeRequestKind::FetchAccountSnapshot { response_tx } => {
                    let snapshot = self.account_snapshot();
                    self.respond_with_latency(response_tx, snapshot);
                }
                MockExchangeRequestKind::FetchBalances { response_tx } => {
                    let balances = self.account.balances().cloned().collect();
                    self.respond_with_latency(response_tx, balances);
                }
                MockExchangeRequestKind::FetchOrdersOpen { response_tx } => {
                    let orders_open = self.account.orders_open().cloned().collect();
                    self.respond_with_latency(response_tx, orders_open);
                }
                MockExchangeRequestKind::FetchTrades {
                    response_tx,
                    time_since,
                } => {
                    let trades = self.account.trades(time_since).cloned().collect();
                    self.respond_with_latency(response_tx, trades);
                }
                MockExchangeRequestKind::CancelOrder {
                    response_tx: _,
                    request,
                } => {
                    error!(
                        exchange = %self.exchange,
                        ?request,
                        "MockExchange received cancel request but only Market orders are supported"
                    );
                }
                MockExchangeRequestKind::OpenOrder {
                    response_tx,
                    request,
                } => {
                    let (response, notifications) = self.open_order(request);
                    self.respond_with_latency(response_tx, response);

                    if let Some(notifications) = notifications {
                        self.account.ack_trade(notifications.trade.clone());
                        self.send_notifications_with_latency(notifications);
                    }
                }
            }
        }

        info!(exchange = %self.exchange, "MockExchange shutting down");
    }

    fn update_time_exchange(&mut self, time_request: DateTime<Utc>) {
        let client_to_exchange_latency = self.latency_ms / 2;

        self.time_exchange_latest = time_request
            .checked_add_signed(TimeDelta::milliseconds(client_to_exchange_latency as i64))
            .unwrap_or(time_request);

        self.account.update_time_exchange(self.time_exchange_latest)
    }

    pub fn time_exchange(&self) -> DateTime<Utc> {
        self.time_exchange_latest
    }

    pub fn account_snapshot(&self) -> UnindexedAccountSnapshot {
        let balances = self.account.balances().cloned().collect();

        let orders_open = self
            .account
            .orders_open()
            .cloned()
            .map(UnindexedOrder::from);

        let orders_cancelled = self
            .account
            .orders_cancelled()
            .cloned()
            .map(UnindexedOrder::from);

        let orders_all = orders_open.chain(orders_cancelled);
        let orders_all = orders_all.sorted_unstable_by_key(|order| order.key.instrument.clone());
        let orders_by_instrument = orders_all.chunk_by(|order| order.key.instrument.clone());

        let instruments = orders_by_instrument
            .into_iter()
            .map(|(instrument, orders)| InstrumentAccountSnapshot {
                instrument,
                orders: orders.into_iter().collect(),
            })
            .collect();

        UnindexedAccountSnapshot {
            exchange: self.exchange,
            balances,
            instruments,
        }
    }

    /// Sends the provided `Response` via the [`oneshot::Sender`] after waiting for the latency
    /// [`Duration`].
    ///
    /// Used to simulate network latency between the exchange and client.
    fn respond_with_latency<Response>(
        &self,
        response_tx: oneshot::Sender<Response>,
        response: Response,
    ) where
        Response: Send + 'static,
    {
        let exchange = self.exchange;
        let latency = std::time::Duration::from_millis(self.latency_ms);

        tokio::spawn(async move {
            tokio::time::sleep(latency).await;
            if response_tx.send(response).is_err() {
                error!(
                    %exchange,
                    kind = std::any::type_name::<Response>(),
                    "MockExchange failed to send oneshot response to client"
                );
            }
        });
    }

    /// Sends the provided `OpenOrderNotifications` via the `MockExchanges`
    /// `broadcast::Sender<UnindexedAccountEvent>` after waiting for the latency
    /// [`Duration`].
    ///
    /// Used to simulate network latency between the exchange and client.
    fn send_notifications_with_latency(&self, notifications: OpenOrderNotifications) {
        let balance = self.build_account_event(notifications.balance);
        let trade = self.build_account_event(notifications.trade);

        let exchange = self.exchange;
        let latency = std::time::Duration::from_millis(self.latency_ms);
        let tx = self.event_tx.clone();
        tokio::spawn(async move {
            tokio::time::sleep(latency).await;

            if tx.send(balance).is_err() {
                error!(
                    %exchange,
                    kind = "Snapshot<AssetBalance<AssetNameExchange>",
                    "MockExchange failed to send AccountEvent notification to client"
                );
            }

            if tx.send(trade).is_err() {
                error!(
                    %exchange,
                    kind = "Trade<QuoteAsset, InstrumentNameExchange>",
                    "MockExchange failed to send AccountEvent notification to client"
                );
            }
        });
    }

    pub fn account_stream(&self) -> BoxStream<'static, UnindexedAccountEvent> {
        futures::StreamExt::boxed(BroadcastStream::new(self.event_tx.subscribe()).map_while(
            |result| match result {
                Ok(event) => Some(event),
                Err(error) => {
                    error!(
                        ?error,
                        "MockExchange Broadcast AccountStream lagged - terminating"
                    );
                    None
                }
            },
        ))
    }

    pub fn cancel_order(
        &mut self,
        _: OrderRequestCancel<ExchangeId, InstrumentNameExchange>,
    ) -> Order<ExchangeId, InstrumentNameExchange, Result<Cancelled, UnindexedOrderError>> {
        unimplemented!()
    }

    pub fn open_order(
        &mut self,
        request: OrderRequestOpen<ExchangeId, InstrumentNameExchange>,
    ) -> (
        Order<ExchangeId, InstrumentNameExchange, Result<Open, UnindexedOrderError>>,
        Option<OpenOrderNotifications>,
    ) {
        if let Err(error) = self.validate_order_kind_supported(request.state.kind) {
            return (build_open_order_err_response(request, error), None);
        }

        let underlying = match self.find_instrument_data(&request.key.instrument) {
            Ok(instrument) => instrument.underlying.clone(),
            Err(error) => return (build_open_order_err_response(request, error), None),
        };

        let time_exchange = self.time_exchange();

        let balance_change_result = match request.state.side {
            Side::Buy => {
                // Buying Instrument requires sufficient QuoteAsset Balance
                let current = self
                    .account
                    .balance_mut(&underlying.quote)
                    .expect("MockExchange has Balance for all configured Instrument assets");

                // Currently we only supported MarketKind orders, so they should be identical
                assert_eq!(current.balance.total, current.balance.free);

                let order_value_quote = request.state.price * request.state.quantity.abs();
                let order_fees_quote = order_value_quote * self.fees_percent;
                let quote_required = order_value_quote + order_fees_quote;

                let maybe_new_balance = current.balance.free - quote_required;

                if maybe_new_balance >= Decimal::ZERO {
                    current.balance.free = maybe_new_balance;
                    current.balance.total = maybe_new_balance;
                    current.time_exchange = time_exchange;

                    Ok((current.clone(), AssetFees::quote_fees(order_fees_quote)))
                } else {
                    Err(ApiError::BalanceInsufficient(
                        underlying.quote,
                        format!(
                            "Available Balance: {}, Required Balance inc. fees: {}",
                            current.balance.free, quote_required
                        ),
                    ))
                }
            }
            Side::Sell => {
                // Selling Instrument requires sufficient BaseAsset Balance
                let current = self
                    .account
                    .balance_mut(&underlying.quote)
                    .expect("MockExchange has Balance for all configured Instrument assets");

                // Currently we only supported MarketKind orders, so they should be identical
                assert_eq!(current.balance.total, current.balance.free);

                let order_value_base = request.state.quantity.abs();
                let order_fees_base = order_value_base * self.fees_percent;
                let base_required = order_value_base + order_fees_base;

                let maybe_new_balance = current.balance.free - base_required;

                if maybe_new_balance >= Decimal::ZERO {
                    current.balance.free = maybe_new_balance;
                    current.balance.total = maybe_new_balance;
                    current.time_exchange = time_exchange;

                    let fees_quote = order_fees_base * request.state.price;

                    Ok((current.clone(), AssetFees::quote_fees(fees_quote)))
                } else {
                    Err(ApiError::BalanceInsufficient(
                        underlying.quote,
                        format!(
                            "Available Balance: {}, Required Balance inc. fees: {}",
                            current.balance.free, base_required
                        ),
                    ))
                }
            }
        };

        let (balance_snapshot, fees) = match balance_change_result {
            Ok((balance_snapshot, fees)) => (Snapshot(balance_snapshot), fees),
            Err(error) => return (build_open_order_err_response(request, error), None),
        };

        let order_id = self.order_id_sequence_fetch_add();
        let trade_id = TradeId(order_id.0.clone());

        let order_response = Order {
            key: request.key.clone(),
            side: request.state.side,
            price: request.state.price,
            quantity: request.state.quantity,
            kind: request.state.kind,
            time_in_force: request.state.time_in_force,
            state: Ok(Open {
                id: order_id.clone(),
                time_exchange: self.time_exchange(),
                filled_quantity: request.state.quantity,
            }),
        };

        let notifications = OpenOrderNotifications {
            balance: balance_snapshot,
            trade: Trade {
                id: trade_id,
                order_id: order_id.clone(),
                instrument: request.key.instrument,
                strategy: request.key.strategy,
                time_exchange: self.time_exchange(),
                side: request.state.side,
                price: request.state.price,
                quantity: request.state.quantity,
                fees,
            },
        };

        (order_response, Some(notifications))
    }

    pub fn validate_order_kind_supported(
        &self,
        order_kind: OrderKind,
    ) -> Result<(), UnindexedOrderError> {
        if order_kind == OrderKind::Market {
            Ok(())
        } else {
            Err(UnindexedOrderError::Rejected(ApiError::OrderRejected(
                format!("MockExchange does not supported OrderKind: {order_kind}"),
            )))
        }
    }

    pub fn find_instrument_data(
        &self,
        instrument: &InstrumentNameExchange,
    ) -> Result<&Instrument<ExchangeId, AssetNameExchange>, UnindexedApiError> {
        self.instruments.get(instrument).ok_or_else(|| {
            ApiError::InstrumentInvalid(
                instrument.clone(),
                format!("MockExchange is not set-up for managing: {instrument}"),
            )
        })
    }

    fn order_id_sequence_fetch_add(&mut self) -> OrderId {
        let sequence = self.order_sequence;
        self.order_sequence += 1;
        OrderId::new(sequence.to_smolstr())
    }

    fn build_account_event<Kind>(&self, kind: Kind) -> UnindexedAccountEvent
    where
        Kind: Into<AccountEventKind<ExchangeId, AssetNameExchange, InstrumentNameExchange>>,
    {
        UnindexedAccountEvent {
            exchange: self.exchange,
            kind: kind.into(),
        }
    }
}

fn build_open_order_err_response<E>(
    request: OrderRequestOpen<ExchangeId, InstrumentNameExchange>,
    error: E,
) -> Order<ExchangeId, InstrumentNameExchange, Result<Open, UnindexedOrderError>>
where
    E: Into<UnindexedOrderError>,
{
    Order {
        key: request.key,
        side: request.state.side,
        price: request.state.price,
        quantity: request.state.quantity,
        kind: request.state.kind,
        time_in_force: request.state.time_in_force,
        state: Err(error.into()),
    }
}

#[derive(Debug)]
pub struct OpenOrderNotifications {
    pub balance: Snapshot<AssetBalance<AssetNameExchange>>,
    pub trade: Trade<QuoteAsset, InstrumentNameExchange>,
}
