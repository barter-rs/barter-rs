use super::{super::channel::GateioChannel, GateioLevel};
use crate::{
    Identifier,
    books::OrderBook,
    event::{MarketEvent, MarketIter},
    exchange::{gateio::message::GateioMessage, subscription::ExchangeSub},
    subscription::book::OrderBookEvent,
};
use barter_instrument::exchange::ExchangeId;
use barter_integration::subscription::SubscriptionId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub type GateioOrderBookL2 = GateioMessage<GateioOrderBookL2Snapshot>;

/// [`Gateio`](super::super::Gateio) OrderBook Level2 snapshot HTTP message.
///
/// Used as the starting [`OrderBook`] before OrderBook Level2 delta WebSocket updates are
/// applied.
///
/// ### Payload Examples
/// See docs: <https://www.gate.io/docs/developers/apiv4/ws/en/#limited-level-full-order-book-snapshot>
/// #### GateioSpot OrderBookL2Snapshot
/// ```json
/// {
///     "t": 1606295412123,
///     "lastUpdateId": 48791820,
///     "s": "BTC_USDT",
///     "l": "5",
///     "bids": [
///      ["19079.55", "0.0195"],
///      ["19079.07", "0.7341"],
///      ["19076.23", "0.00011808"],
///      ["19073.9", "0.105"],
///      ["19068.83", "0.1009"]
///    ],
///     "asks": [
///      ["19080.24", "0.1638"],
///      ["19080.91", "0.1366"],
///      ["19080.92", "0.01"],
///      ["19081.29", "0.01"],
///      ["19083.8", "0.097"]
/// }
/// ```

#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct GateioOrderBookL2Snapshot {
    #[serde(alias = "s", deserialize_with = "de_ob_l2_subscription_id")]
    pub subscription_id: SubscriptionId,
    #[serde(default, rename = "t", with = "chrono::serde::ts_milliseconds_option")]
    pub time_engine: Option<DateTime<Utc>>,
    #[serde(rename = "lastUpdateId")]
    pub last_update_id: u64,
    pub l: String,
    #[serde(rename = "bids")]
    pub bids: Vec<GateioLevel>,
    #[serde(rename = "asks")]
    pub asks: Vec<GateioLevel>,
}

impl Identifier<Option<SubscriptionId>> for GateioOrderBookL2 {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.data.subscription_id.clone())
    }
}

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, GateioOrderBookL2)>
    for MarketIter<InstrumentKey, OrderBookEvent>
{
    fn from(
        (exchange, instrument, message): (ExchangeId, InstrumentKey, GateioOrderBookL2),
    ) -> Self {
        let time_received = Utc::now();
        Self(vec![Ok(MarketEvent {
            time_exchange: message.data.time_engine.unwrap_or(time_received),
            time_received: time_received,
            exchange,
            instrument,
            kind: OrderBookEvent::from(message),
        })])
    }
}

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, GateioOrderBookL2)>
    for MarketEvent<InstrumentKey, OrderBookEvent>
{
    fn from(
        (exchange, instrument, snapshot): (ExchangeId, InstrumentKey, GateioOrderBookL2),
    ) -> Self {
        let time_received = Utc::now();
        Self {
            time_exchange: snapshot.data.time_engine.unwrap_or(time_received),
            time_received,
            exchange,
            instrument,
            kind: OrderBookEvent::from(snapshot),
        }
    }
}

impl From<GateioOrderBookL2> for OrderBookEvent {
    fn from(snapshot: GateioOrderBookL2) -> Self {
        Self::Snapshot(OrderBook::new(
            snapshot.data.last_update_id,
            snapshot.data.time_engine,
            snapshot.data.bids,
            snapshot.data.asks,
        ))
    }
}

pub fn de_ob_l2_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    <&str as Deserialize>::deserialize(deserializer)
        .map(|market| ExchangeSub::from((GateioChannel::ORDER_BOOK_L2, market)).id())
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;
        use rust_decimal_macros::dec;

        #[test]
        fn test_gateio_order_book_l2_snapshot() {
            struct TestCase {
                input: &'static str,
                expected: GateioOrderBookL2Snapshot,
            }

            let tests = vec![TestCase {
                // TC0: valid Spot GateioOrderBookL2Snapshot
                input: r#"
                    {   "s":"BTC_USDT",
                        "lastUpdateId": 1027024,
                        "l":"100",
                        "bids": [
                            [
                                "4.00000000",
                                "431.00000000"
                            ]
                        ],
                        "asks": [
                            [
                                "4.00000200",
                                "12.00000000"
                            ]
                        ]
                    }
                    "#,
                expected: GateioOrderBookL2Snapshot {
                    subscription_id: SubscriptionId::from("spot.order_book|BTC_USDT"),
                    last_update_id: 1027024,
                    time_engine: Default::default(),
                    l: "100".to_string(),
                    bids: vec![GateioLevel {
                        price: dec!(4.00000000),
                        amount: dec!(431.00000000),
                    }],
                    asks: vec![GateioLevel {
                        price: dec!(4.00000200),
                        amount: dec!(12.00000000),
                    }],
                },
            }];

            for (index, test) in tests.into_iter().enumerate() {
                assert_eq!(
                    serde_json::from_str::<GateioOrderBookL2Snapshot>(test.input).unwrap(),
                    test.expected,
                    "TC{} failed",
                    index
                );
            }
        }
    }
}
