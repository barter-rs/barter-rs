use crate::{
    books::OrderBook,
    event::{MarketEvent, MarketIter},
    exchange::okx::trade::OkxMessage,
    subscription::book::OrderBookEvent,
};
use barter_instrument::exchange::ExchangeId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::OkxLevel;

/// [`Okx`](super::super::Okx) real-time OrderBook Level2 message.
///
/// ### Raw Payload Examples  
/// #### OkxSpot OrderBookL2 (books channel)
/// See docs: <https://www.okx.com/docs-v5/en/#websocket-api-public-channel-order-book-channel>
/// ```json
/// {
///   "arg": {
///     "channel": "books",
///     "instId": "BTC-USDT"
///   },
///   "data": [
///     {
///       "asks": [
///         ["41010.2", "0.60067239", "0", "2"],
///         ["41010.3", "0.30000000", "0", "1"]
///       ],
///       "bids": [
///         ["41009.9", "0.01", "0", "1"],
///         ["41009.8", "0.05", "0", "1"]
///       ],
///       "ts": "1629966436396",
///       "checksum": -855196043,
///       "prevSeqId": 123,
///       "seqId": 124
///     }
///   ],
///   "action": "update"
/// }
/// ```
pub type OkxOrderBookL2 = OkxMessage<OkxOrderBookL2Inner>;

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct OkxOrderBookL2Inner {
    #[serde(default)]
    pub asks: Vec<OkxLevel>,
    #[serde(default)]
    pub bids: Vec<OkxLevel>,
    #[serde(
        rename = "ts",
        deserialize_with = "barter_integration::de::de_str_u64_epoch_ms_as_datetime_utc"
    )]
    pub time: DateTime<Utc>,
    #[serde(default)]
    pub checksum: Option<i64>,
    #[serde(rename = "seqId", default)]
    pub sequence_id: Option<u64>,
    #[serde(rename = "prevSeqId", default)]
    pub prev_sequence_id: Option<u64>,
}

impl<InstrumentKey: Clone> From<(ExchangeId, InstrumentKey, OkxOrderBookL2)>
    for MarketIter<InstrumentKey, OrderBookEvent>
{
    fn from(
        (exchange, instrument, message): (ExchangeId, InstrumentKey, OkxOrderBookL2),
    ) -> Self {
        message.data
            .into_iter()
            .map(|book_data| {
                let orderbook = OrderBook::new(
                    book_data.sequence_id.unwrap_or(0),
                    Some(book_data.time),
                    book_data.bids,
                    book_data.asks,
                );

                // OKX sends incremental updates, so treat all as updates
                // Full snapshots are only sent on initial subscription or reconnection
                let kind = OrderBookEvent::Update(orderbook);

                Ok(MarketEvent {
                    time_exchange: book_data.time,
                    time_received: Utc::now(),
                    exchange,
                    instrument: instrument.clone(),
                    kind,
                })
            })
            .collect()
    }
}

impl Default for OkxOrderBookL2Inner {
    fn default() -> Self {
        Self {
            asks: Vec::new(),
            bids: Vec::new(),
            time: Utc::now(),
            checksum: None,
            sequence_id: None,
            prev_sequence_id: None,
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
        fn test_okx_order_book_l2() {
            let input = r#"
            {
                "arg": {
                    "channel": "books",
                    "instId": "BTC-USDT"
                },
                "data": [
                    {
                        "asks": [
                            ["41010.2", "0.60067239"],
                            ["41010.3", "0.30000000"]
                        ],
                        "bids": [
                            ["41009.9", "0.01"],
                            ["41009.8", "0.05"]
                        ],
                        "ts": "1629966436396",
                        "checksum": -855196043,
                        "seqId": 124
                    }
                ]
            }
            "#;

            let actual = serde_json::from_str::<OkxOrderBookL2>(input).unwrap();
            
            assert_eq!(actual.subscription_id, SubscriptionId::from("books|BTC-USDT"));
            assert_eq!(actual.data.len(), 1);
            
            let book_data = &actual.data[0];
            assert_eq!(book_data.asks.len(), 2);
            assert_eq!(book_data.bids.len(), 2);
            assert_eq!(book_data.checksum, Some(-855196043));
            assert_eq!(book_data.sequence_id, Some(124));
            
            assert_eq!(book_data.asks[0].price, dec!(41010.2));
            assert_eq!(book_data.asks[0].amount, dec!(0.60067239));
            assert_eq!(book_data.bids[0].price, dec!(41009.9));
            assert_eq!(book_data.bids[0].amount, dec!(0.01));
        }

        #[test]
        fn test_okx_order_book_l2_snapshot() {
            let input = r#"
            {
                "arg": {
                    "channel": "books",
                    "instId": "ETH-USDT"
                },
                "data": [
                    {
                        "asks": [["2550.1", "1.5"]],
                        "bids": [["2549.9", "2.0"]],
                        "ts": "1629966436400",
                        "seqId": 1
                    }
                ]
            }
            "#;

            let actual = serde_json::from_str::<OkxOrderBookL2>(input).unwrap();
            assert_eq!(actual.data[0].sequence_id, Some(1));
        }
    }
}