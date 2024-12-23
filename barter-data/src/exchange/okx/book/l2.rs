use super::OkxLevel;
use crate::{
    exchange::okx::trade::de_okx_message_arg_as_subscription_id,
    subscription::book::{OrderBook, OrderBookSide},
    Identifier,
};
use barter_integration::model::{Side, SubscriptionId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub enum OkxOrderBookAction {
    #[serde(rename = "snapshot")]
    SNAPSHOT,
    #[serde(rename = "update")]
    UPDATE,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct OkxOrderBookDataL2 {
    #[serde(
        alias = "ts",
        deserialize_with = "barter_integration::de::de_str_u64_epoch_ms_as_datetime_utc"
    )]
    pub time: DateTime<Utc>,
    pub asks: Vec<OkxLevel>,
    pub bids: Vec<OkxLevel>,
    pub checksum: i64,
    #[serde(rename = "prevSeqId")]
    pub prev_seq_id: i64,
    #[serde(rename = "seqId")]
    pub seq_id: i64,
}

impl From<OkxOrderBookDataL2> for OrderBook {
    fn from(snapshot: OkxOrderBookDataL2) -> Self {
        Self {
            last_update_time: Utc::now(),
            bids: OrderBookSide::new(Side::Buy, snapshot.bids),
            asks: OrderBookSide::new(Side::Sell, snapshot.asks),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct OkxFuturesOrderBookL2 {
    #[serde(
        rename = "arg",
        deserialize_with = "de_okx_message_arg_as_subscription_id"
    )]
    pub subscription_id: SubscriptionId,
    pub action: OkxOrderBookAction,
    pub data: Vec<OkxOrderBookDataL2>,
}

impl Identifier<Option<SubscriptionId>> for OkxFuturesOrderBookL2 {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;

        #[test]
        fn test_okx_order_book_l2() {
            let input = r#"
            {
              "arg": {
                "channel": "books",
                "instId": "BTC-USDT"
              },
              "action": "snapshot",
              "data": [
                {
                  "asks": [
                    ["8476.98", "415", "0", "13"]
                  ],
                  "bids": [
                    ["8476.97", "256", "0", "12"]
                  ],
                  "ts": "1597026383085",
                  "checksum": 123,
                  "prevSeqId": 123,
                  "seqId": 123456
                }
              ]
            }
            "#;

            assert_eq!(
                serde_json::from_str::<OkxFuturesOrderBookL2>(input).unwrap(),
                OkxFuturesOrderBookL2 {
                    subscription_id: SubscriptionId::from("books|BTC-USDT"),
                    action: OkxOrderBookAction::SNAPSHOT,
                    data: vec![OkxOrderBookDataL2 {
                        time: DateTime::<Utc>::from_timestamp_millis(1597026383085).unwrap(),
                        asks: vec![OkxLevel {
                            price: 8476.98,
                            amount: 415.0,
                        }],
                        bids: vec![OkxLevel {
                            price: 8476.97,
                            amount: 256.0,
                        }],
                        seq_id: 123456,
                        checksum: 123,
                        prev_seq_id: 123
                    }]
                }
            )
        }
    }
}
