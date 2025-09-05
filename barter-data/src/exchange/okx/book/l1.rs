use crate::{
    books::Level,
    event::{MarketEvent, MarketIter},
    exchange::okx::trade::OkxMessage,
    subscription::book::OrderBookL1,
};
use barter_instrument::exchange::ExchangeId;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// [`Okx`](super::super::Okx) real-time OrderBook Level1 (top of books) message.
///
/// ### Raw Payload Examples
/// #### OkxSpot OrderBookL1 (BBO-TBT channel)
/// See docs: <https://www.okx.com/docs-v5/en/#websocket-api-public-channel-bbo-tbt-channel>
/// ```json
/// {
///   "arg": {
///     "channel": "bbo-tbt",
///     "instId": "BTC-USDT"
///   },
///   "data": [
///     {
///       "asks": [["41006.8", "0.60038239", "0", "1"]],
///       "bids": [["41006.7", "0.01", "0", "1"]],
///       "ts": "1629966436396",
///       "checksum": -855196043
///     }
///   ]
/// }
/// ```
pub type OkxOrderBookL1 = OkxMessage<OkxOrderBookL1Inner>;

#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct OkxOrderBookL1Inner {
    #[serde(default)]
    pub asks: Vec<OkxL1Level>,
    #[serde(default)]
    pub bids: Vec<OkxL1Level>,
    #[serde(
        rename = "ts",
        deserialize_with = "barter_integration::de::de_str_u64_epoch_ms_as_datetime_utc"
    )]
    pub time: DateTime<Utc>,
    #[serde(default)]
    pub checksum: Option<i64>,
}

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, OkxOrderBookL1)>
    for MarketIter<InstrumentKey, OrderBookL1>
{
    fn from(
        (exchange_id, instrument, book): (ExchangeId, InstrumentKey, OkxOrderBookL1),
    ) -> Self {
        // OKX should always send at least one data element for L1
        let Some(book_data) = book.data.into_iter().next() else {
            // Return empty event with current time if no data
            return Self(vec![Ok(MarketEvent {
                time_exchange: Utc::now(),
                time_received: Utc::now(),
                exchange: exchange_id,
                instrument,
                kind: OrderBookL1 {
                    last_update_time: Utc::now(),
                    best_bid: None,
                    best_ask: None,
                },
            })]);
        };
        
        let best_ask = book_data.asks
            .first()
            .filter(|level| !level.price.is_zero())
            .map(|level| Level::new(level.price, level.amount));

        let best_bid = book_data.bids
            .first()
            .filter(|level| !level.price.is_zero())
            .map(|level| Level::new(level.price, level.amount));

        Self(vec![Ok(MarketEvent {
            time_exchange: book_data.time,
            time_received: Utc::now(),
            exchange: exchange_id,
            instrument,
            kind: OrderBookL1 {
                last_update_time: book_data.time,
                best_bid,
                best_ask,
            },
        })])
    }
}

impl Default for OkxOrderBookL1Inner {
    fn default() -> Self {
        Self {
            asks: Vec::new(),
            bids: Vec::new(),
            time: Utc::now(),
            checksum: None,
        }
    }
}

/// OKX Level 1 order book level with additional metadata.
///
/// #### Raw Payload Examples
/// See docs: <https://www.okx.com/docs-v5/en/#websocket-api-public-channel-bbo-tbt-channel>
/// ```json
/// ["41006.8", "0.60038239", "0", "1"]
/// ```
/// 
/// Format: [price, size, deprecated, num_orders]
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct OkxL1Level {
    #[serde(with = "rust_decimal::serde::str")]
    pub price: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount: Decimal,
    #[serde(with = "rust_decimal::serde::str", default)]
    pub deprecated: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub num_orders: Decimal,
}

impl From<OkxL1Level> for Level {
    fn from(level: OkxL1Level) -> Self {
        Self {
            price: level.price,
            amount: level.amount,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;
        use rust_decimal_macros::dec;
        use barter_integration::subscription::SubscriptionId;

        #[test]
        fn test_okx_l1_level() {
            let input = r#"["41006.8", "0.60038239", "0", "1"]"#;
            assert_eq!(
                serde_json::from_str::<OkxL1Level>(input).unwrap(),
                OkxL1Level {
                    price: dec!(41006.8),
                    amount: dec!(0.60038239),
                    deprecated: dec!(0),
                    num_orders: dec!(1),
                },
            )
        }

        #[test]
        fn test_okx_order_book_l1() {
            let input = r#"
            {
                "arg": {
                    "channel": "bbo-tbt",
                    "instId": "BTC-USDT"
                },
                "data": [
                    {
                        "asks": [["41006.8", "0.60038239", "0", "1"]],
                        "bids": [["41006.7", "0.01", "0", "1"]],
                        "ts": "1629966436396",
                        "checksum": -855196043
                    }
                ]
            }
            "#;

            let actual = serde_json::from_str::<OkxOrderBookL1>(input).unwrap();
            
            assert_eq!(actual.subscription_id, SubscriptionId::from("bbo-tbt|BTC-USDT"));
            assert_eq!(actual.data.len(), 1);
            
            let book_data = &actual.data[0];
            assert_eq!(book_data.asks.len(), 1);
            assert_eq!(book_data.bids.len(), 1);
            assert_eq!(book_data.checksum, Some(-855196043));
            
            assert_eq!(book_data.asks[0].price, dec!(41006.8));
            assert_eq!(book_data.bids[0].price, dec!(41006.7));
        }
    }
}