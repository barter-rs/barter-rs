use crate::{
    event::{MarketEvent, MarketIter},
    exchange::{coinbase::channel::CoinbaseChannel, subscription::ExchangeSub, ExchangeId},
    books::Level,
    subscription::book::OrderBookL1,
    Identifier,
};
use barter_integration::subscription::SubscriptionId;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// [`Coinbase`](super::super::Coinbase) real-time OrderBook Level1 (top of book) message.
///
/// ### Raw Payload Examples
/// #### Coinbase OrderBookL1
/// See docs: <https://docs.cdp.coinbase.com/exchange/docs/websocket-channels/#level2-channel>
/// ```json
///{
///  "type": "ticker",
///  "sequence": 37475248783,
///  "product_id": "ETH-USD",
///  "price": "1285.22",
///  "open_24h": "1310.79",
///  "volume_24h": "245532.79269678",
///  "low_24h": "1280.52",
///  "high_24h": "1313.8",
///  "volume_30d": "9788783.60117027",
///  "best_bid": "1285.04",
///  "best_bid_size": "0.46688654",
///  "best_ask": "1285.27",
///  "best_ask_size": "1.56637040",
///  "side": "buy",
///  "time": "2022-10-19T23:28:22.061769Z",
///  "trade_id": 370843401,
///  "last_size": "11.4396987"
///}
/// ```
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct CoinbaseOrderBookL1 {
    #[serde(rename(deserialize = "time"))]
    pub time: DateTime<Utc>,
    #[serde(rename(deserialize = "type"))]
    pub kind: String,
    pub sequence: u64,
    #[serde(
        rename(deserialize = "product_id"),
        deserialize_with = "de_ob_l1_subscription_id"
    )]
    pub subscription_id: SubscriptionId,
    #[serde(
        rename(deserialize = "best_bid"),
        deserialize_with = "barter_integration::de::de_str"
    )]
    pub best_bid_price: Decimal,
    #[serde(
        rename(deserialize = "best_bid_size"),
        with = "rust_decimal::serde::str"
    )]
    pub best_bid_amount: Decimal,
    #[serde(
        rename(deserialize = "best_ask"),
        with = "rust_decimal::serde::str"
    )]
    pub best_ask_price: Decimal,
    #[serde(
        rename(deserialize = "best_ask_size"),
        with = "rust_decimal::serde::str"
    )]
    pub best_ask_amount: Decimal,
}

impl Identifier<Option<SubscriptionId>> for CoinbaseOrderBookL1 {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}

impl<InstrumentId> From<(ExchangeId, InstrumentId, CoinbaseOrderBookL1)>
    for MarketIter<InstrumentId, OrderBookL1>
{
    fn from(
        (exchange_id, instrument, book): (ExchangeId, InstrumentId, CoinbaseOrderBookL1),
    ) -> Self {
        let best_ask = if book.best_ask_price.is_zero() {
            None
        } else {
            Some(Level::new(book.best_ask_price, book.best_ask_amount))
        };

        let best_bid = if book.best_bid_price.is_zero() {
            None
        } else {
            Some(Level::new(book.best_bid_price, book.best_bid_amount))
        };

        Self(vec![Ok(MarketEvent {
            time_exchange: book.time,
            time_received: Utc::now(),
            exchange: exchange_id,
            instrument,
            kind: OrderBookL1 {
                last_update_time: book.time,
                best_bid,
                best_ask,
            },
        })])
    }
}

/// Deserialize a [`CoinbaseOrderBookL1`] "s" (eg/ "BTCUSDT") as the associated [`SubscriptionId`].
///
/// eg/ "ticker|BTC-USD"
pub fn de_ob_l1_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    <&str as Deserialize>::deserialize(deserializer)
        .map(|market| ExchangeSub::from((CoinbaseChannel::ORDER_BOOK_L1, market)).id())
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;

        #[test]
        fn test_coinbase_order_book_l1() {
            struct TestCase {
                input: &'static str,
                expected: CoinbaseOrderBookL1,
            }

            let time = Utc::now();

            let tests = vec![TestCase {
                // TC0: valid Spot CoinbaseOrderBookL1
                input: r#"
                {
                  "type": "ticker",
                  "sequence": 37475248783,
                  "product_id": "ETH-USD",
                  "price": "1285.22",
                  "open_24h": "1310.79",
                  "volume_24h": "245532.79269678",
                  "low_24h": "1280.52",
                  "high_24h": "1313.8",
                  "volume_30d": "9788783.60117027",
                  "best_bid": "1285.04",
                  "best_bid_size": "0.46688654",
                  "best_ask": "1285.27",
                  "best_ask_size": "1.56637040",
                  "side": "buy",
                  "time": "2022-10-19T23:28:22.061769Z",
                  "trade_id": 370843401,
                  "last_size": "11.4396987"
                }
              "#,
                expected: CoinbaseOrderBookL1 {
                    kind: "ticker".into(),
                    sequence: 37475248783,
                    subscription_id: SubscriptionId::from("ticker|ETH-USD"),
                    time,
                    best_bid_price: Decimal::try_from(1285.04).unwrap(),
                    best_bid_amount: Decimal::try_from(0.46688654).unwrap(),
                    best_ask_price: Decimal::try_from(1285.27).unwrap(),
                    best_ask_amount: Decimal::try_from(1.56637040).unwrap(),
                },
            }];

            for (index, test) in tests.into_iter().enumerate() {
                let actual = serde_json::from_str::<CoinbaseOrderBookL1>(test.input).unwrap();
                let actual = CoinbaseOrderBookL1 { time, ..actual };
                assert_eq!(actual, test.expected, "TC{} failed", index);
            }
        }
    }
}
