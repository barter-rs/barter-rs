use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures_util::future::try_join_all;
use super::super::book::BinanceLevel;
use crate::Identifier;
use barter_integration::model::{Exchange, SubscriptionId};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;
use barter_integration::error::SocketError;
use barter_integration::protocol::websocket::WsMessage;
use barter_integration::Transformer;
use crate::books::OrderBook;
use crate::error::DataError;
use crate::event::{MarketEvent, MarketIter};
use crate::exchange::binance::book::l2::BinanceOrderBookL2Snapshot;
use crate::exchange::binance::futures::BinanceFuturesUsd;
use crate::exchange::binance::market::BinanceMarket;
use crate::exchange::{Connector, ExchangeId};
use crate::instrument::InstrumentData;
use crate::subscription::book::{OrderBookEvent, OrderBooksL2};
use crate::subscription::{Map, Subscription};
use crate::transformer::ExchangeTransformer;

/// [`BinanceFuturesUsd`] HTTP OrderBook L2 snapshot url.
///
/// See docs: <https://binance-docs.github.io/apidocs/futures/en/#order-book>
pub const HTTP_BOOK_L2_SNAPSHOT_URL_BINANCE_FUTURES_USD: &str = "https://fapi.binance.com/fapi/v1/depth";

/// Todo: rust docs & do I want to add exchange specific sequence validation?
#[derive(Debug)]
pub struct BinanceFuturesUsdOrderBooksL2Transformer<InstrumentKey> {
    instrument_map: Map<InstrumentKey>
}

#[async_trait]
impl<InstrumentKey> ExchangeTransformer<BinanceFuturesUsd, InstrumentKey, OrderBooksL2>
for BinanceFuturesUsdOrderBooksL2Transformer<InstrumentKey>
where
    InstrumentKey: Clone + Send,
{
    async fn init(_: UnboundedSender<WsMessage>, instrument_map: Map<InstrumentKey>) -> Result<Self, DataError> {
        Ok(Self { instrument_map })
    }

    async fn fetch_snapshots<Instrument>(
        subscriptions: &[Subscription<BinanceFuturesUsd, Instrument, OrderBooksL2>]
    ) -> Result<Vec<MarketEvent<InstrumentKey, OrderBookEvent>>, DataError>
    where
        Instrument: InstrumentData<Key= InstrumentKey>,
        Subscription<BinanceFuturesUsd, Instrument, OrderBooksL2>: Identifier<BinanceMarket>
    {
        let l2_snapshot_futures = subscriptions
            .iter()
            .map(|sub| {
                // Construct initial OrderBook snapshot GET url
                let market = sub.id();
                let snapshot_url = format!(
                    "{}?symbol={}&limit=100",
                    HTTP_BOOK_L2_SNAPSHOT_URL_BINANCE_FUTURES_USD,
                    market.0,
                );

                async move {
                    // Fetch initial OrderBook snapshot via HTTP
                    let snapshot = reqwest::get(snapshot_url)
                        .await
                        .map_err(SocketError::Http)?
                        .json::<BinanceOrderBookL2Snapshot>()
                        .await
                        .map_err(SocketError::Http)?;

                    Ok(MarketEvent::from((
                        ExchangeId::BinanceFuturesUsd,
                        sub.instrument.key().clone(),
                        snapshot
                    )))
                }
            });

        try_join_all(l2_snapshot_futures).await
    }
}

impl<InstrumentKey> Transformer for BinanceFuturesUsdOrderBooksL2Transformer<InstrumentKey>
where
    InstrumentKey: Clone,
{
    type Error = DataError;
    type Input = BinanceFuturesOrderBookL2Update;
    type Output = MarketEvent<InstrumentKey, OrderBookEvent>;
    type OutputIter = Vec<Result<Self::Output, Self::Error>>;

    fn transform(&mut self, input: Self::Input) -> Self::OutputIter {
        // Determine if the message has an identifiable SubscriptionId
        let subscription_id = match input.id() {
            Some(subscription_id) => subscription_id,
            None => return vec![],
        };

        // Find Instrument associated with Input and transform
        match self.instrument_map.find(&subscription_id) {
            Ok(instrument) => {
                MarketIter::<InstrumentKey, OrderBookEvent>::from((
                    BinanceFuturesUsd::ID,
                    instrument.clone(),
                    input,
                )).0
            }
            Err(unidentifiable) => vec![Err(DataError::Socket(unidentifiable))],
        }
    }
}


/// [`BinanceFuturesUsd`] OrderBook Level2 deltas WebSocket message.
///
/// ### Raw Payload Examples
/// See docs: <https://binance-docs.github.io/apidocs/futures/en/#partial-book-depth-streams>
/// ```json
/// {
///     "e": "depthUpdate",
///     "E": 123456789,
///     "T": 123456788,
///     "s": "BTCUSDT",
///     "U": 157,
///     "u": 160,
///     "pu": 149,
///     "b": [
///         ["0.0024", "10"]
///     ],
///     "a": [
///         ["0.0026", "100"]
///     ]
/// }
/// ```
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BinanceFuturesOrderBookL2Update {
    #[serde(
        alias = "s",
        deserialize_with = "super::super::book::l2::de_ob_l2_subscription_id"
    )]
    pub subscription_id: SubscriptionId,
    #[serde(
        alias = "E",
        deserialize_with = "barter_integration::de::de_u64_epoch_ms_as_datetime_utc"
    )]
    pub time_exchange: DateTime<Utc>,
    #[serde(
        alias = "T",
        deserialize_with = "barter_integration::de::de_u64_epoch_ms_as_datetime_utc"
    )]
    pub time_engine: DateTime<Utc>,
    #[serde(alias = "U")]
    pub first_update_id: u64,
    #[serde(alias = "u")]
    pub last_update_id: u64,
    #[serde(alias = "pu")]
    pub prev_last_update_id: u64,
    #[serde(alias = "b")]
    pub bids: Vec<BinanceLevel>,
    #[serde(alias = "a")]
    pub asks: Vec<BinanceLevel>,
}

impl Identifier<Option<SubscriptionId>> for BinanceFuturesOrderBookL2Update {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, BinanceFuturesOrderBookL2Update)> for MarketIter<InstrumentKey, OrderBookEvent> {
    fn from((exchange, instrument, update): (ExchangeId, InstrumentKey, BinanceFuturesOrderBookL2Update)) -> Self {
        Self(vec![Ok(MarketEvent {
            time_exchange: update.time_exchange,
            time_received: Utc::now(),
            exchange: Exchange::from(exchange),
            instrument,
            kind: OrderBookEvent::Update(OrderBook::new(
                Some(update.last_update_id),
                Some(update.time_engine),
                update.bids,
                update.asks,
            )),
        })])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_de_binance_futures_order_book_l2_update() {
        let input = r#"
            {
                "e": "depthUpdate",
                "E": 123456789,
                "T": 123456788,
                "s": "BTCUSDT",
                "U": 157,
                "u": 160,
                "pu": 149,
                "b": [
                    [
                        "0.0024",
                        "10"
                    ]
                ],
                "a": [
                    [
                        "0.0026",
                        "100"
                    ]
                ]
            }
        "#;

        assert_eq!(
            serde_json::from_str::<BinanceFuturesOrderBookL2Update>(input).unwrap(),
            BinanceFuturesOrderBookL2Update {
                subscription_id: SubscriptionId::from("@depth@100ms|BTCUSDT"),
                first_update_id: 157,
                last_update_id: 160,
                prev_last_update_id: 149,
                bids: vec![BinanceLevel {
                    price: dec!(0.0024),
                    amount: dec!(10.0)
                },],
                asks: vec![BinanceLevel {
                    price: dec!(0.0026),
                    amount: dec!(100.0)
                },]
            }
        );
    }
}
