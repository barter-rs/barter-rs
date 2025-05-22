use url::Url;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::str::FromStr;
use crate::{
    client::ExecutionClient,
    UnindexedAccountEvent, UnindexedAccountSnapshot,
    balance::{AssetBalance, Balance},
    error::{UnindexedClientError, UnindexedOrderError},
    order::{
        Order, OrderKey, OrderKind, TimeInForce,
        id::{ClientOrderId, OrderId, StrategyId},
        request::{OrderRequestCancel, OrderRequestOpen, UnindexedOrderResponseCancel},
        state::{Open, OrderState},
    },
    trade::{Trade, AssetFees, TradeId},
};
use jackbot_instrument::{
    Side,
    asset::{name::AssetNameExchange, QuoteAsset},
    exchange::ExchangeId,
    instrument::name::InstrumentNameExchange,
};
use jackbot_integration::protocol::websocket::{connect, WebSocket};
use jackbot_integration::snapshot::Snapshot;

#[derive(Clone, Debug)]
pub struct CoinbaseWsConfig {
    pub url: Url,
    pub auth_payload: String,
}

#[derive(Clone, Debug)]
pub struct CoinbaseWsClient {
    config: CoinbaseWsConfig,
}

impl ExecutionClient for CoinbaseWsClient {
    const EXCHANGE: ExchangeId = ExchangeId::Coinbase;
    type Config = CoinbaseWsConfig;
    type AccountStream = UnboundedReceiverStream<UnindexedAccountEvent>;

    fn new(config: Self::Config) -> Self {
        Self { config }
    }

    async fn account_snapshot(
        &self,
        _assets: &[AssetNameExchange],
        _instruments: &[InstrumentNameExchange],
    ) -> Result<UnindexedAccountSnapshot, UnindexedClientError> {
        Ok(UnindexedAccountSnapshot {
            exchange: Self::EXCHANGE,
            balances: vec![],
            instruments: vec![],
        })
    }

    async fn account_stream(
        &self,
        _assets: &[AssetNameExchange],
        _instruments: &[InstrumentNameExchange],
    ) -> Result<Self::AccountStream, UnindexedClientError> {
        let (tx, rx) = mpsc::unbounded_channel();
        let url = self.config.url.clone();
        let auth = self.config.auth_payload.clone();
        tokio::spawn(async move {
            loop {
                match connect(url.clone()).await {
                    Ok(ws) => {
                        if run_connection(ws, &tx, &auth).await.is_err() {
                            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                            continue;
                        } else {
                            break;
                        }
                    }
                    Err(_) => {
                        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                    }
                }
            }
        });
        Ok(UnboundedReceiverStream::new(rx))
    }

    async fn cancel_order(
        &self,
        _request: OrderRequestCancel<ExchangeId, &InstrumentNameExchange>,
    ) -> UnindexedOrderResponseCancel {
        unimplemented!()
    }

    async fn open_order(
        &self,
        _request: OrderRequestOpen<ExchangeId, &InstrumentNameExchange>,
    ) -> Order<ExchangeId, InstrumentNameExchange, Result<Open, UnindexedOrderError>> {
        unimplemented!()
    }

    async fn fetch_balances(&self) -> Result<Vec<AssetBalance<AssetNameExchange>>, UnindexedClientError> {
        unimplemented!()
    }

    async fn fetch_open_orders(&self) -> Result<Vec<Order<ExchangeId, InstrumentNameExchange, Open>>, UnindexedClientError> {
        unimplemented!()
    }

    async fn fetch_trades(
        &self,
        _time_since: DateTime<Utc>,
    ) -> Result<Vec<Trade<QuoteAsset, InstrumentNameExchange>>, UnindexedClientError> {
        unimplemented!()
    }
}

async fn run_connection(
    mut ws: WebSocket,
    tx: &mpsc::UnboundedSender<UnindexedAccountEvent>,
    auth: &str,
) -> Result<(), ()> {
    if ws.send(WsMessage::Text(auth.to_string())).await.is_err() {
        return Err(());
    }
    while let Some(msg) = ws.next().await {
        let msg = match msg {
            Ok(m) => m,
            Err(_) => return Err(()),
        };
        match msg {
            WsMessage::Text(text) => {
                if let Ok(event) = serde_json::from_str::<CoinbaseEvent>(&text) {
                    if let Some(evt) = to_account_event(event) {
                        let _ = tx.send(evt);
                    }
                }
            }
            WsMessage::Close(_) => return Err(()),
            _ => {}
        }
    }
    Err(())
}

#[derive(serde::Deserialize)]
#[serde(tag = "type")]
enum CoinbaseEvent {
    #[serde(rename = "balance")]
    Balance {
        time: u64,
        asset: String,
        free: String,
        total: String,
    },
    #[serde(rename = "order")]
    Order {
        time: u64,
        product_id: String,
        side: String,
        price: String,
        size: String,
        order_id: String,
        status: String,
    },
    #[serde(rename = "fill")]
    Fill {
        time: u64,
        trade_id: u64,
        product_id: String,
        side: String,
        price: String,
        size: String,
    },
}

fn to_account_event(event: CoinbaseEvent) -> Option<UnindexedAccountEvent> {
    match event {
        CoinbaseEvent::Balance { time, asset, free, total } => {
            let time = Utc.timestamp_millis_opt(time as i64).single()?;
            let free = Decimal::from_str(&free).ok()?;
            let total = Decimal::from_str(&total).ok()?;
            let balance = AssetBalance {
                asset: AssetNameExchange(asset),
                balance: Balance { total, free },
                time_exchange: time,
            };
            Some(crate::AccountEvent::new(
                ExchangeId::Coinbase,
                crate::AccountEventKind::BalanceSnapshot(Snapshot(balance)),
            ))
        }
        CoinbaseEvent::Order { time, product_id, side, price, size, order_id, .. } => {
            let time = Utc.timestamp_millis_opt(time as i64).single()?;
            let side = match side.to_uppercase().as_str() {
                "BUY" => Side::Buy,
                "SELL" => Side::Sell,
                _ => return None,
            };
            let price = Decimal::from_str(&price).ok()?;
            let quantity = Decimal::from_str(&size).ok()?;
            let order = Order {
                key: OrderKey {
                    exchange: ExchangeId::Coinbase,
                    instrument: InstrumentNameExchange(product_id),
                    strategy: StrategyId::unknown(),
                    cid: ClientOrderId::default(),
                },
                side,
                price,
                quantity,
                kind: OrderKind::Market,
                time_in_force: TimeInForce::GoodUntilCancelled { post_only: false },
                state: OrderState::active(Open {
                    id: OrderId(order_id),
                    time_exchange: time,
                    filled_quantity: quantity,
                }),
            };
            Some(crate::AccountEvent::new(
                ExchangeId::Coinbase,
                crate::AccountEventKind::OrderSnapshot(Snapshot(order)),
            ))
        }
        CoinbaseEvent::Fill { time, trade_id, product_id, side, price, size } => {
            let time = Utc.timestamp_millis_opt(time as i64).single()?;
            let side = match side.to_uppercase().as_str() {
                "BUY" => Side::Buy,
                "SELL" => Side::Sell,
                _ => return None,
            };
            let price = Decimal::from_str(&price).ok()?;
            let quantity = Decimal::from_str(&size).ok()?;
            let trade = Trade {
                id: TradeId(trade_id.to_string()),
                order_id: OrderId(String::new()),
                instrument: InstrumentNameExchange(product_id),
                strategy: StrategyId::unknown(),
                time_exchange: time,
                side,
                price,
                quantity,
                fees: AssetFees::default(),
            };
            Some(crate::AccountEvent::new(
                ExchangeId::Coinbase,
                crate::AccountEventKind::Trade(trade),
            ))
        }
    }
}

