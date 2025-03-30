use barter_instrument::{
    asset::{name::AssetNameExchange, QuoteAsset},
    exchange::ExchangeId,
    instrument::name::InstrumentNameExchange,
};
use barter_integration::{
    channel::{mpsc_unbounded, Tx},
    protocol::{
        http::{private::encoder::HexEncoder, rest::client::RestClient},
        websocket::{connect, WebSocketParser},
        StreamParser,
    },
};
use chrono::{DateTime, Utc};
use futures::StreamExt;
use hmac::{Hmac, Mac};
use http::{
    parser::BybitParser,
    requests::{PlaceOrderBody, PlaceOrderRequest},
    signer::{BybitRequestSigner, BybitSigner},
};
use itertools::Itertools;
use rust_decimal::{prelude::FromPrimitive, Decimal};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_tungstenite::tungstenite::Error;
use tracing::{error, warn};
use types::BybitOrderStatus;
use websocket::{subscribe, BybitPayload, OrderExecutionData, OrderUpdateData};

use crate::{
    balance::AssetBalance,
    error::{ConnectivityError, UnindexedClientError},
    order::{Cancelled, ClientOrderId, Open, Order, RequestCancel, RequestOpen, StrategyId},
    trade::{AssetFees, Trade},
    AccountEvent, AccountEventKind, ApiCredentials, InstrumentAccountSnapshot,
    UnindexedAccountEvent, UnindexedAccountSnapshot,
};

use super::ExecutionClient;

mod http;
mod types;
mod websocket;

const WEBSOCKET_BASE_URL_BYBIT: &str = "wss://stream.bybit.com/v5/private";
const HTTP_BASE_URL_BYBIT: &str = "https://api.bybit.com";

#[derive(Debug, Clone)]
pub struct BybitConfig {
    credentials: ApiCredentials,
}

#[derive(Debug, Clone)]
pub struct Bybit {
    config: BybitConfig,
    rest_client: RestClient<'static, BybitRequestSigner, BybitParser>,
}

impl ExecutionClient for Bybit {
    const EXCHANGE: ExchangeId = ExchangeId::BybitSpot;
    type Config = BybitConfig;
    type AccountStream = UnboundedReceiverStream<UnindexedAccountEvent>;

    fn new(config: Self::Config) -> Self {
        let hmac = Hmac::new_from_slice(config.credentials.secret.as_bytes())
            .expect("ApiCredentials secret invalid length");

        Self {
            rest_client: RestClient::new(
                HTTP_BASE_URL_BYBIT,
                BybitRequestSigner::new(
                    BybitSigner::new(config.credentials.key.clone()),
                    hmac,
                    HexEncoder,
                ),
                BybitParser,
            ),
            config,
        }
    }

    async fn account_snapshot(
        &self,
        _: &[AssetNameExchange],
        instruments: &[InstrumentNameExchange],
    ) -> Result<UnindexedAccountSnapshot, UnindexedClientError> {
        let balances = self.fetch_balances().await?;

        let orders_by_instrument = self
            .fetch_open_orders()
            .await?
            .into_iter()
            .sorted_by(|a, b| a.instrument.cmp(&b.instrument))
            .chunk_by(|order| order.instrument.clone());

        let instruments = orders_by_instrument
            .into_iter()
            .fold(Vec::with_capacity(instruments.len()), |mut snapshots, (instrument, orders)| {
                if !instruments.contains(&instrument) {
                    warn!(
                        exchange = %Self::EXCHANGE,
                        %instrument,
                        "BinanceSpot | AccountSnapshot | received open_orders for untracked instrument - filtering"
                    );

                    return snapshots
                }

                snapshots.push(InstrumentAccountSnapshot {
                    instrument,
                    orders: orders
                        .into_iter()
                        .map(Order::from)
                        .collect(),
                });

                snapshots
            });

        Ok(UnindexedAccountSnapshot {
            exchange: Self::EXCHANGE,
            balances,
            instruments,
        })
    }

    async fn account_stream(
        &self,
        assets: &[AssetNameExchange],
        instruments: &[InstrumentNameExchange],
    ) -> Result<Self::AccountStream, UnindexedClientError> {
        let mut stream = connect(WEBSOCKET_BASE_URL_BYBIT)
            .await
            .map_err(|err| ConnectivityError::Socket(err.to_string()))?;

        // Authenticate connection and subscribe to the required topics.
        subscribe(&self.config.credentials, &mut stream)
            .await
            .map_err(|err| ConnectivityError::Socket(err.to_string()))?;

        // Channel used to push account updates
        let (events_tx, events_rx) = mpsc_unbounded::<UnindexedAccountEvent>();

        // Handle events from the client
        tokio::spawn({
            let assets = assets.to_vec();
            let instruments = instruments.to_vec();

            async move {
                while let Some(message) = stream.next().await {
                    // TODO: Log error that we received

                    let Some(parsed) = WebSocketParser::parse::<BybitPayload>(message) else {
                        continue;
                    };

                    let message = match parsed {
                        Ok(message) => message,
                        Err(err) => {
                            error!(
                                ?err,
                                "received error from the ByBit execution client stream"
                            );
                            // TODO: Should we cancel the stream?
                            return;
                        }
                    };

                    match extract_event(message, &assets, &instruments).await {
                        // The event should be published
                        Ok(Some(event)) => {
                            events_tx.send(event).unwrap();
                        }
                        // Event was filtered out
                        Ok(None) => {
                            todo!()
                        }
                        // Error
                        Err(err) => {
                            // TODO: Check if the error should cancel the stream
                            error!(
                                ?err,
                                "error occurred while handling Bybit execution client message"
                            );
                            return;
                        }
                    }
                }
            }
        });

        Ok(events_rx.into_stream())
    }

    async fn cancel_order(
        &self,
        request: Order<ExchangeId, &InstrumentNameExchange, RequestCancel>,
    ) -> Order<ExchangeId, InstrumentNameExchange, Result<Cancelled, UnindexedClientError>> {
        todo!()
    }

    async fn open_order(
        &self,
        order_request: Order<ExchangeId, &InstrumentNameExchange, RequestOpen>,
    ) -> Order<ExchangeId, InstrumentNameExchange, Result<Open, UnindexedClientError>> {
        let request = PlaceOrderRequest::new(PlaceOrderBody {
            category: todo!(),
            symbol: todo!(),
            side: todo!(),
            kind: todo!(),
            time_in_force: todo!(),
            quantity: todo!(),
            price: todo!(),
            position_side: todo!(),
            client_order_id: todo!(),
            reduce_only: todo!(),
        });

        let state = self
            .rest_client
            .execute(request)
            .await
            .map(|(response, _metric)| Open {
                id: response.result.exchange_order_id,
                time_exchange: response.time,
                price: order_request.state.price,
                quantity: order_request.state.quantity,
                filled_quantity: Decimal::ZERO,
            })
            .map_err(Into::into);

        Order {
            exchange: Self::EXCHANGE,
            instrument: *order_request.instrument,
            strategy: order_request.strategy,
            cid: order_request.cid,
            side: order_request.side,
            state,
        }
    }

    async fn fetch_balances(
        &self,
    ) -> Result<Vec<AssetBalance<AssetNameExchange>>, UnindexedClientError> {
        todo!()
    }

    async fn fetch_open_orders(
        &self,
    ) -> Result<Vec<Order<ExchangeId, InstrumentNameExchange, Open>>, UnindexedClientError> {
        todo!()
    }

    async fn fetch_trades(
        &self,
        time_since: DateTime<Utc>,
    ) -> Result<Vec<Trade<QuoteAsset, InstrumentNameExchange>>, UnindexedClientError> {
        todo!()
    }
}

async fn extract_event(
    payload: BybitPayload,
    _assets: &[AssetNameExchange],
    instruments: &[InstrumentNameExchange],
) -> Result<Option<UnindexedAccountEvent>, Error> {
    let event = match payload.topic.as_str() {
        "order" => {
            let order = serde_json::from_str::<OrderUpdateData>(payload.data.get()).unwrap();

            instruments
                .contains(&order.symbol)
                .then(|| to_unified_order(order, payload.timestamp))
        }
        "execution" => {
            let execution = serde_json::from_str::<OrderExecutionData>(payload.data.get()).unwrap();

            instruments
                .contains(&execution.symbol)
                .then(|| to_unified_execution(execution, payload.timestamp))
                .flatten()
        }
        // TODO: Add balance and position updates
        _ => {
            error!(?payload, "message from unknown topic received");
            None
        }
    };

    Ok(event)
}

fn to_unified_order(order: OrderUpdateData, time_exchange: DateTime<Utc>) -> UnindexedAccountEvent {
    let kind = match order.status {
        BybitOrderStatus::New
        | BybitOrderStatus::PartiallyFilled
        | BybitOrderStatus::Untriggered => {
            AccountEventKind::OrderOpened::<ExchangeId, AssetNameExchange, InstrumentNameExchange>(
                Order {
                    exchange: ExchangeId::BybitSpot,
                    instrument: order.symbol,
                    strategy: StrategyId::unknown(),
                    cid: ClientOrderId::new(order.client_order_id.unwrap()),
                    side: order.side,
                    state: Ok(Open {
                        id: order.exchange_order_id,
                        time_exchange,
                        price: Decimal::from_f64(order.original_price).unwrap(),
                        // TODO: We should probably also add an average price
                        quantity: Decimal::from_f64(order.original_quantity).unwrap(),
                        filled_quantity: Decimal::from_f64(order.cumulative_executed_quantity)
                            .unwrap(),
                    }),
                },
            )
        }
        BybitOrderStatus::Rejected
        | BybitOrderStatus::PartiallyFilledCanceled
        | BybitOrderStatus::Filled
        | BybitOrderStatus::Cancelled
        | BybitOrderStatus::Triggered
        | BybitOrderStatus::Deactivated => {
            AccountEventKind::OrderCancelled::<ExchangeId, AssetNameExchange, InstrumentNameExchange>(
                Order {
                    exchange: ExchangeId::BybitSpot,
                    instrument: order.symbol,
                    strategy: StrategyId::unknown(),
                    cid: ClientOrderId::new(order.client_order_id.unwrap()),
                    side: order.side,
                    state: Ok(Cancelled {
                        id: order.exchange_order_id,
                        time_exchange,
                    }),
                },
            )
        }
    };

    AccountEvent {
        exchange: ExchangeId::BybitSpot,
        kind,
    }
}

pub fn to_unified_execution(
    execution: OrderExecutionData,
    time_exchange: DateTime<Utc>,
) -> Option<UnindexedAccountEvent> {
    Some(AccountEvent {
        exchange: ExchangeId::BybitSpot,
        kind: AccountEventKind::Trade(Trade {
            id: execution.trade_id,
            order_id: execution.exchange_order_id,
            instrument: execution.symbol,
            strategy: StrategyId::unknown(),
            time_exchange,
            // TODO: Handle side
            side: todo!(),
            price: Decimal::from_f64(execution.exec_price)?,
            quantity: Decimal::from_f64(execution.exec_qty)?,
            // TODO: Handle fees
            fees: AssetFees {
                asset: QuoteAsset,
                fees: todo!(),
            },
        }),
    })
}
