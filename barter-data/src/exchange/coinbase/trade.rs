use super::CoinbaseChannel;
use crate::{
    Identifier,
    event::{MarketEvent, MarketIter},
    exchange::ExchangeSub,
    subscription::trade::PublicTrade,
};
use barter_instrument::{Side, exchange::ExchangeId};
use barter_integration::subscription::SubscriptionId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Coinbase real-time trade WebSocket message.
///
/// ### Raw Payload Examples
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-channels#match>
/// ```json
/// {
///     "type": "match",
///     "trade_id": 10,
///     "sequence": 50,
///     "maker_order_id": "ac928c66-ca53-498f-9c13-a110027a60e8",
///     "taker_order_id": "132fb6ae-456b-4654-b4e0-d681ac05cea1",
///     "time": "2014-11-07T08:19:27.028459Z",
///     "product_id": "BTC-USD",
///     "size": "5.23512",
///     "price":
///     "400.23",
///     "side": "sell"
/// }
/// ```
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct CoinbaseTrade {
    #[serde(alias = "product_id", deserialize_with = "de_trade_subscription_id")]
    pub subscription_id: SubscriptionId,
    #[serde(alias = "trade_id")]
    pub id: u64,
    pub time: DateTime<Utc>,
    #[serde(alias = "size", deserialize_with = "barter_integration::de::de_str")]
    pub amount: f64,
    #[serde(deserialize_with = "barter_integration::de::de_str")]
    pub price: f64,
    pub side: Side,
}

impl Identifier<Option<SubscriptionId>> for CoinbaseTrade {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, CoinbaseTrade)>
    for MarketIter<InstrumentKey, PublicTrade>
{
    fn from((exchange_id, instrument, trade): (ExchangeId, InstrumentKey, CoinbaseTrade)) -> Self {
        Self(vec![Ok(MarketEvent {
            time_exchange: trade.time,
            time_received: Utc::now(),
            exchange: exchange_id,
            instrument,
            kind: PublicTrade {
                id: trade.id.to_string(),
                price: trade.price,
                amount: trade.amount,
                side: trade.side,
            },
        })])
    }
}

/// Deserialize a [`CoinbaseTrade`] "product_id" (eg/ "BTC-USD") as the associated [`SubscriptionId`]
/// (eg/ SubscriptionId("matches|BTC-USD").
pub fn de_trade_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    <&str as Deserialize>::deserialize(deserializer)
        .map(|product_id| ExchangeSub::from((CoinbaseChannel::TRADES, product_id)).id())
}

#[cfg(test)]
mod tests {
    use super::*;
    use barter_integration::error::SocketError;
    use chrono::NaiveDateTime;
    use serde::de::Error;
    use std::str::FromStr;

    #[test]
    fn test_de_coinbase_trade() {
        struct TestCase {
            input: &'static str,
            expected: Result<CoinbaseTrade, SocketError>,
        }

        let cases = vec![
            TestCase {
                // TC0: invalid Coinbase message w/ unknown tag
                input: r#"{"type": "unknown", "sequence": 50,"product_id": "BTC-USD"}"#,
                expected: Err(SocketError::Deserialise {
                    error: serde_json::Error::custom(""),
                    payload: "".to_owned(),
                }),
            },
            TestCase {
                // TC1: valid Spot CoinbaseTrade
                input: r#"
                {
                    "type": "match","trade_id": 10,"sequence": 50,
                    "maker_order_id": "ac928c66-ca53-498f-9c13-a110027a60e8",
                    "taker_order_id": "132fb6ae-456b-4654-b4e0-d681ac05cea1",
                    "time": "2014-11-07T08:19:27.028459Z",
                    "product_id": "BTC-USD", "size": "5.23512", "price": "400.23", "side": "sell"
                }"#,
                expected: Ok(CoinbaseTrade {
                    subscription_id: SubscriptionId::from("matches|BTC-USD"),
                    id: 10,
                    price: 400.23,
                    amount: 5.23512,
                    side: Side::Sell,
                    time: NaiveDateTime::from_str("2014-11-07T08:19:27.028459")
                        .unwrap()
                        .and_utc(),
                }),
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = serde_json::from_str::<CoinbaseTrade>(test.input);
            match (actual, test.expected) {
                (Ok(actual), Ok(expected)) => {
                    assert_eq!(actual, expected, "TC{} failed", index)
                }
                (Err(_), Err(_)) => {
                    // Test passed
                }
                (actual, expected) => {
                    // Test failed
                    panic!(
                        "TC{index} failed because actual != expected. \nActual: {actual:?}\nExpected: {expected:?}\n"
                    );
                }
            }
        }
    }
}
