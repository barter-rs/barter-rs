use barter_instrument::Side;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

/// ### Raw Payload Examples
/// See docs: <https://www.bitmex.com/app/wsAPI#Response-Format>
///
/// #### Trade payload
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
///```
#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct BitmexTrade {
    pub timestamp: DateTime<Utc>,
    pub symbol: SmolStr,
    pub side: Side,
    #[serde(rename = "size")]
    pub amount: f64,
    pub price: f64,

    #[serde(rename = "trdMatchID")]
    pub id: SmolStr,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone};
    use smol_str::ToSmolStr;

    #[test]
    fn test_bitmex_trade() {
        let input = r#"
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
            "#;

        assert_eq!(
            serde_json::from_str::<BitmexTrade>(input).unwrap(),
            BitmexTrade {
                timestamp: Utc.with_ymd_and_hms(2023, 2, 18, 9, 27, 59).unwrap()
                    + Duration::milliseconds(701),
                symbol: "XBTUSD".to_smolstr(),
                side: Side::Sell,
                amount: 200.0,
                price: 24564.5,
                id: "31e50cb7-e005-a44e-f354-86e88dff52eb".to_smolstr(),
            }
        );
    }
}
