use super::OkxLevel;
use crate::{
    exchange::okx::trade::de_okx_message_arg_as_subscription_id, subscription::book::OrderBookL1,
    Identifier,
};
use barter_integration::model::SubscriptionId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct OkxOrderBookDataL1 {
    #[serde(
        alias = "ts",
        deserialize_with = "barter_integration::de::de_str_u64_epoch_ms_as_datetime_utc"
    )]
    pub time: DateTime<Utc>,
    pub asks: Vec<OkxLevel>,
    pub bids: Vec<OkxLevel>,
    #[serde(rename = "seqId")]
    pub seq_id: i64,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct OkxFuturesOrderBookL1 {
    #[serde(
        rename = "arg",
        deserialize_with = "de_okx_message_arg_as_subscription_id"
    )]
    pub subscription_id: SubscriptionId,
    pub data: Vec<OkxOrderBookDataL1>,
}

impl Identifier<Option<SubscriptionId>> for OkxFuturesOrderBookL1 {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}

impl From<OkxOrderBookDataL1> for OrderBookL1 {
    fn from(data: OkxOrderBookDataL1) -> Self {
        Self {
            last_update_time: Utc::now(),
            best_bid: data.bids[0].into(),
            best_ask: data.asks[0].into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;

        #[test]
        fn test_okx_order_book_l1() {
            let input = r#"
            {
              "arg": {
                "channel": "bbo-tbt",
                "instId": "BCH-USDT-SWAP"
              },
              "data": [
                {
                  "asks": [
                    [
                      "111.06","55154","0","2"
                    ]
                  ],
                  "bids": [
                    [
                      "111.05","57745","0","2"
                    ]
                  ],
                  "ts": "1670324386802",
                  "seqId": 363996337
                }
              ]
            }
            "#;

            assert_eq!(
                serde_json::from_str::<OkxFuturesOrderBookL1>(input).unwrap(),
                OkxFuturesOrderBookL1 {
                    subscription_id: SubscriptionId::from("bbo-tbt|BCH-USDT-SWAP"),
                    data: vec![OkxOrderBookDataL1 {
                        time: DateTime::<Utc>::from_timestamp_millis(1670324386802).unwrap(),
                        asks: vec![OkxLevel {
                            price: 111.06,
                            amount: 55154.0,
                        }],
                        bids: vec![OkxLevel {
                            price: 111.05,
                            amount: 57745.0,
                        }],
                        seq_id: 363996337,
                    }]
                }
            )
        }
    }
}
