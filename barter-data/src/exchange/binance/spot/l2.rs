use async_trait::async_trait;
use super::super::book::BinanceLevel;
use crate::books::OrderBook;
use crate::event::{MarketEvent, MarketIter};
use crate::exchange::{Connector, ExchangeId};
use crate::subscription::book::{OrderBookEvent, OrderBooksL2};
use crate::Identifier;
use barter_integration::model::{Exchange, SubscriptionId};
use chrono::{DateTime, Utc};
use futures_util::future::try_join_all;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;
use barter_integration::error::SocketError;
use barter_integration::protocol::websocket::WsMessage;
use barter_integration::Transformer;
use crate::error::DataError;
use crate::exchange::binance::book::l2::BinanceOrderBookL2Snapshot;
use crate::exchange::binance::market::BinanceMarket;
use crate::exchange::binance::spot::BinanceSpot;
use crate::instrument::InstrumentData;
use crate::subscription::{Map, Subscription};
use crate::transformer::ExchangeTransformer;

/// [`BinanceSpot`](super::BinanceSpot) HTTP OrderBook L2 snapshot url.
///
/// See docs: <https://binance-docs.github.io/apidocs/spot/en/#order-book>
pub const HTTP_BOOK_L2_SNAPSHOT_URL_BINANCE_SPOT: &str = "https://api.binance.com/api/v3/depth";

pub struct BinanceSpotOrderBooksL2Transformer<InstrumentKey> {
    instrument_map: Map<InstrumentKey>,
}

#[async_trait]
impl<InstrumentKey> ExchangeTransformer<BinanceSpot, InstrumentKey, OrderBooksL2>
for BinanceSpotOrderBooksL2Transformer<InstrumentKey>
where
    InstrumentKey: Clone + Send,
{
    async fn init(_: UnboundedSender<WsMessage>, instrument_map: Map<InstrumentKey>) -> Result<Self, DataError> {
        Ok(Self { instrument_map })
    }

    async fn fetch_snapshots<Instrument>(
        subscriptions: &[Subscription<BinanceSpot, Instrument, OrderBooksL2>]
    ) -> Result<Vec<MarketEvent<InstrumentKey, OrderBookEvent>>, DataError>
    where
        Instrument: InstrumentData<Key = InstrumentKey>,
        Subscription<BinanceSpot, Instrument, OrderBooksL2>: Identifier<BinanceMarket>
    {
        let l2_snapshot_futures = subscriptions
            .iter()
            .map(|sub| {
                // Construct initial OrderBook snapshot GET url
                let market = sub.id();

                let snapshot_url = format!(
                    "{}?symbol={}&limit=100",
                    HTTP_BOOK_L2_SNAPSHOT_URL_BINANCE_SPOT,
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
                        ExchangeId::BinanceSpot,
                        sub.instrument.key().clone(),
                        snapshot
                    )))
                }
            });

        try_join_all(l2_snapshot_futures).await
    }
}

impl<InstrumentKey> Transformer for BinanceSpotOrderBooksL2Transformer<InstrumentKey>
where
    InstrumentKey: Clone,
{
    type Error = DataError;
    type Input = BinanceSpotOrderBookL2Update;
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
                    BinanceSpot::ID,
                    instrument.clone(),
                    input,
                )).0
            }
            Err(unidentifiable) => vec![Err(DataError::Socket(unidentifiable))],
        }
    }
}

/// [`BinanceSpot`] OrderBook Level2 deltas WebSocket message.
///
/// ### Raw Payload Examples
/// See docs: <https://binance-docs.github.io/apidocs/spot/en/#partial-book-depth-streams>
/// ```json
/// {
///     "e":"depthUpdate",
///     "E":1671656397761,
///     "s":"ETHUSDT",
///     "U":22611425143,
///     "u":22611425151,
///     "b":[
///         ["1209.67000000","85.48210000"],
///         ["1209.66000000","20.68790000"]
///     ],
///     "a":[]
/// }
/// ```
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BinanceSpotOrderBookL2Update {
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
    #[serde(alias = "U")]
    pub first_update_id: u64,
    #[serde(alias = "u")]
    pub last_update_id: u64,
    #[serde(alias = "b")]
    pub bids: Vec<BinanceLevel>,
    #[serde(alias = "a")]
    pub asks: Vec<BinanceLevel>,
}

impl Identifier<Option<SubscriptionId>> for BinanceSpotOrderBookL2Update {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, BinanceSpotOrderBookL2Update)>
    for MarketIter<InstrumentKey, OrderBookEvent>
{
    fn from(
        (exchange_id, instrument, update): (ExchangeId, InstrumentKey, BinanceSpotOrderBookL2Update),
    ) -> Self {
        Self(vec![Ok(MarketEvent {
            time_exchange: update.time_exchange,
            time_received: Utc::now(),
            exchange: Exchange::from(exchange_id),
            instrument,
            kind: OrderBookEvent::Update(OrderBook::new(
                Some(update.last_update_id),
                None,
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
    fn test_de_binance_spot_order_book_l2_update() {
        let input = r#"
            {
                "e":"depthUpdate",
                "E":1671656397761,
                "s":"ETHUSDT",
                "U":22611425143,
                "u":22611425151,
                "b":[
                    ["1209.67000000","85.48210000"],
                    ["1209.66000000","20.68790000"]
                ],
                "a":[]
            }
            "#;

        assert_eq!(
            serde_json::from_str::<BinanceSpotOrderBookL2Update>(input).unwrap(),
            BinanceSpotOrderBookL2Update {
                subscription_id: SubscriptionId::from("@depth@100ms|ETHUSDT"),
                time_exchange: DateTime::from_timestamp_millis(1671656397761).unwrap(),
                first_update_id: 22611425143,
                last_update_id: 22611425151,
                bids: vec![
                    BinanceLevel {
                        price: dec!(1209.67000000),
                        amount: dec!(85.48210000)
                    },
                    BinanceLevel {
                        price: dec!(1209.66000000),
                        amount: dec!(20.68790000)
                    },
                ],
                asks: vec![]
            }
        );
    }
}
