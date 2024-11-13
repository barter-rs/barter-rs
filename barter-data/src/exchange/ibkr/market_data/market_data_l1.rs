use crate::{
    books::Level,
    event::{MarketEvent, MarketIter},
    exchange::{ibkr::channel::IbkrChannel, subscription::ExchangeSub, ExchangeId},
    subscription::book::OrderBookL1,
    Identifier,
};
use barter_integration::subscription::SubscriptionId;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// [`Ibkr`](super::Ibkr) market data websocket subscription.
///
/// ### Request
///
/// ```text
/// smd+conId+{"fields": ["field_1", "field_2", "field_n", "field_n+1"]}
/// ```
///
/// ### Response
/// ```json
/// {
///     "server_id": "server_id",
///     "conidEx": "conidEx",
///     "conid": conid,
///     _updated": "_updated",
///     "6119": "serverId",
///     "field_1": "field_1",
///     "field_2": "field_2",
///     "field_n": "field_n",
///     "field_n+1": "field_n+1",
///     "6509": "RB",
///     "topic": "smd+conid"
/// }
/// ```
///
/// See docs: <https://ibkrcampus.com/ibkr-api-page/cpapi-v1/#ws-sub-watchlist-data>
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct IbkrMarketDataL1 {
    #[serde(alias = "conidEx", deserialize_with = "de_ob_l1_subscription_id")]
    pub subscription_id: SubscriptionId,
    #[serde(
        rename = "_updated",
        deserialize_with = "barter_integration::de::de_u64_epoch_ms_as_datetime_utc"
    )]
    pub last_update_time: DateTime<Utc>,
    #[serde(rename = "31", with = "rust_decimal::serde::str_option")]
    pub last_price: Option<Decimal>,
    #[serde(rename = "84", default, with = "rust_decimal::serde::str_option")]
    pub best_bid_price: Option<Decimal>,
    #[serde(rename = "88", default, with = "rust_decimal::serde::str_option")]
    pub best_bid_size: Option<Decimal>,
    #[serde(rename = "86", default, with = "rust_decimal::serde::str_option")]
    pub best_ask_price: Option<Decimal>,
    #[serde(rename = "85", default, with = "rust_decimal::serde::str_option")]
    pub best_ask_size: Option<Decimal>,
}

impl Identifier<Option<SubscriptionId>> for IbkrMarketDataL1 {
    fn id(&self) -> Option<SubscriptionId> {
        Some(SubscriptionId::from(self.subscription_id.clone()))
    }
}

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, IbkrMarketDataL1)>
    for MarketIter<InstrumentKey, OrderBookL1>
{
    fn from((exchange_id, instrument, md): (ExchangeId, InstrumentKey, IbkrMarketDataL1)) -> Self {
        Self(vec![Ok(MarketEvent {
            time_exchange: md.last_update_time,
            time_received: Utc::now(),
            exchange: exchange_id,
            instrument,
            kind: OrderBookL1 {
                last_update_time: md.last_update_time,
                best_bid: Level::new(md.best_bid_price.unwrap_or_default(), md.best_bid_size.unwrap_or_default()),
                best_ask: Level::new(md.best_ask_price.unwrap_or_default(), md.best_ask_size.unwrap_or_default()),
            },
        })])
    }
}

/// Deserialize a [`IbkrMarketDataL1`] "conid" (eg/ "265598") as the associated [`SubscriptionId`].
///
pub fn de_ob_l1_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    <&str as Deserialize>::deserialize(deserializer)
        .map(|market| ExchangeSub::from((IbkrChannel::ORDER_BOOK_L1.sub_type, market)).id())
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;
        use barter_integration::{de::datetime_utc_from_epoch_duration, error::SocketError};
        use rust_decimal_macros::dec;
        use std::time::Duration;

        #[test]
        fn test_ibkr_message_md() {
            // TODO: fill in `input` with valid data
            let input = r#"
            {
                "server_id": "server_id",
                "conidEx": "1234",
                "conid": 1234,
                "_updated": 1630048897897,
                "6119": "serverId",
                "31": "100.99",
                "84": "100.90",
                "88": "1200",
                "86": "101.90",
                "85": "1500",
                "6509":"RB",
                "topic":"smd+1234"
            }
            "#;

            let actual = serde_json::from_str::<IbkrMarketDataL1>(input);
            let expected: Result<IbkrMarketDataL1, SocketError> = Ok(IbkrMarketDataL1 {
                subscription_id: SubscriptionId::from("md|1234"),
                last_price: Some(dec!(100.99)),
                best_bid_price: Some(dec!(100.9)),
                best_bid_size: Some(dec!(1200)),
                best_ask_price: Some(dec!(101.9)),
                best_ask_size: Some(dec!(1500)),
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
