use crate::exchange::bitmex::websocket::{
    error::BitmexError,
    info::BitmexInfo,
    response::{BitmexResponseSubscribe, BitmexResponseUnsubscribe},
    trade::BitmexTrade,
};
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

pub mod error;
pub mod info;
pub mod request;
pub mod response;
pub mod trade;

pub const BITMEX_WS_BASE_URL: &str = "wss://ws.bitmex.com/realtime";
pub const BITMEX_WS_BASE_URL_PLATFORM: &str = "wss://ws.bitmex.com/realtimePlatform";

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(untagged, rename_all = "camelCase")]
pub enum BitmexMessage<T = BitmexData> {
    Response(BitmexResponse),
    Event(BitmexEvent<T>),
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(untagged, rename_all = "camelCase")]
pub enum BitmexResponse {
    Subscribe(BitmexResponseSubscribe),
    Unsubscribe(BitmexResponseUnsubscribe),
    Error(BitmexError),
    Info(BitmexInfo),
}

/// ## Bitmex Event From An Active Subscription
///
/// ### Raw Payload Examples
/// #### BitmexTrade
/// ```json
/// {
///     "table": "trade",
///     "action": "insert",
///     "data": [
///         {
///             "timestamp": "2023-02-18T09:27:59.701Z",
///             "symbol": "XBTUSD",
///             "side": "Sell",
///             "size": 200,
///             "price": 24564.5,
///             "tickDirection": "MinusTick",
///             "trdMatchID": "31e50cb7-e005-a44e-f354-86e88dff52eb",
///             "grossValue": 814184,
///             "homeNotional": 0.00814184,
///             "foreignNotional": 200,
///             "trdType": "Regular"
///         }
///     ]
/// }
/// ```
#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct BitmexEvent<T = BitmexData> {
    pub table: SmolStr,
    pub action: BitmexAction,
    pub data: Vec<T>,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BitmexAction {
    Partial,
    Insert,
    Delete,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(untagged, rename_all = "camelCase")]
pub enum BitmexData {
    Trade(BitmexTrade),
}

#[cfg(test)]
mod tests {
    use super::*;
    use barter_instrument::Side;
    use chrono::{Duration, TimeZone, Utc};
    use smol_str::ToSmolStr;

    #[test]
    fn test_de_bitmex_payload() {
        let input = r#"
        {
            "table": "trade",
            "action": "insert",
            "data": [
                {
                    "timestamp": "2023-02-18T09:27:59.701Z",
                    "symbol": "XBTUSD",
                    "side": "Sell",
                    "size": 200,
                    "price": 24564.5,
                    "tickDirection": "MinusTick",
                    "trdMatchID": "31e50cb7-e005-a44e-f354-86e88dff52eb",
                    "grossValue": 814184,
                    "homeNotional": 0.00814184,
                    "foreignNotional": 200,
                    "trdType": "Regular"
                }
            ]
        }
        "#;

        assert_eq!(
            serde_json::from_str::<BitmexEvent<BitmexTrade>>(input).unwrap(),
            BitmexEvent {
                table: "trade".to_smolstr(),
                action: BitmexAction::Insert,
                data: vec![BitmexTrade {
                    timestamp: Utc.with_ymd_and_hms(2023, 2, 18, 9, 27, 59).unwrap()
                        + Duration::milliseconds(701),
                    symbol: "XBTUSD".to_smolstr(),
                    side: Side::Sell,
                    amount: 200.0,
                    price: 24564.5,
                    id: "31e50cb7-e005-a44e-f354-86e88dff52eb".to_smolstr(),
                }]
            },
        );
    }
}
