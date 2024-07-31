use crate::{
    event::{MarketEvent, MarketIter},
    exchange::ExchangeId,
    subscription::book::{Level, OrderBookL1},
    Identifier,
};
use barter_integration::model::{Exchange, SubscriptionId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// [`Ibkr`](super::Ibkr) market data websocket subscription.
///
/// ### Request
///
/// ```json
/// smd+conId+{
///   "fields": [
///     "field_1",
///     "field_2",
///     "field_n",
///     "field_n+1"
///   ]
/// }
/// ```
///
/// ### Response
/// ```json
/// {
///     "server_id":"server_id",
///     "conidEx":"conidEx",
///     "conid":"conid",
///     _updated":"_updated",
///     "6119":"serverId",
///     "field_1":"field_1",
///     "field_2":"field_2",
///     "field_n":"field_n",
///     "field_n+1":"field_n+1",
///     "6509":"RB",
///     "topic":"smd+conid"
/// }
/// ```
///
/// See docs: <https://ibkrcampus.com/ibkr-api-page/cpapi-v1/#ws-sub-watchlist-data>
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct IbkrMarketDataL1 {
    #[serde(alias = "conid")]
    pub contract_id: i32,
    #[serde(
        rename = "_updated",
        deserialize_with = "barter_integration::de::de_u64_epoch_ms_as_datetime_utc"
    )]
    pub last_update_time: DateTime<Utc>,
    #[serde(rename = "84")]
    pub best_bid_price: Option<f64>,
    #[serde(rename = "88")]
    pub best_bid_size: Option<u32>,
    #[serde(rename = "86")]
    pub best_ask_price: Option<f64>,
    #[serde(rename = "85")]
    pub best_ask_size: Option<u32>,
}

impl Identifier<Option<SubscriptionId>> for IbkrMarketDataL1 {
    fn id(&self) -> Option<SubscriptionId> {
        Some(SubscriptionId::from(self.contract_id.to_string()))
    }
}

impl<InstrumentId> From<(ExchangeId, InstrumentId, IbkrMarketDataL1)> for MarketIter<InstrumentId, OrderBookL1> {
    fn from((exchange_id, instrument, md): (ExchangeId, InstrumentId, IbkrMarketDataL1)) -> Self {
        Self(vec![Ok(MarketEvent {
            exchange_time: md.last_update_time,
            received_time: Utc::now(),
            exchange: Exchange::from(exchange_id),
            instrument,
            kind: OrderBookL1 {
                last_update_time: md.last_update_time,
                best_bid: Level::new(
                    md.best_bid_price.unwrap_or(0.0),
                    f64::from(md.best_bid_size.unwrap_or(0)) / 100.0
                ),
                best_ask: Level::new(
                    md.best_ask_price.unwrap_or(0.0),
                    f64::from(md.best_ask_size.unwrap_or(0)) / 100.0
                ),
            },
        })])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;
        use barter_integration::{de::datetime_utc_from_epoch_duration, error::SocketError};
        use std::time::Duration;

        #[test]
        fn test_ibkr_message_md() {
            // TODO: fill in `input` with valid data
            let input = r#"
            {
                "server_id":"server_id",
                "conidEx":"conidEx",
                "conid":1234,
                "_updated":"_updated",
                "6119":"serverId",
                "field_1":"field_1",
                "field_2":"field_2",
                "field_n":"field_n",
                "field_n+1":"field_n+1",
                "6509":"RB",
                "topic":"smd+conid"
            }
            "#;

            let actual = serde_json::from_str::<IbkrMarketDataL1>(input);
            let expected: Result<IbkrMarketDataL1, SocketError> = Ok(IbkrMarketDataL1 {
                contract_id: 1234,
                best_bid_price: Some(100.9),
                best_bid_size: Some(1200),
                best_ask_price: Some(101.9),
                best_ask_size: Some(1500),
                last_update_time: datetime_utc_from_epoch_duration(Duration::from_millis(
                    1630048897897,
                )),
            });

            match (actual, expected) {
                (Ok(actual), Ok(expected)) => {
                    assert_eq!(actual, expected, "TC failed")
                }
                (Err(_), Err(_)) => {
                    // Test passed
                }
                (actual, expected) => {
                    // Test failed
                    panic!("TC failed because actual != expected. \nActual: {actual:?}\nExpected: {expected:?}\n");
                }
            }
        }
    }
}
