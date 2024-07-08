use serde::{Deserialize, Serialize};

/// [`Gateio`](super::Gateio) WebSocket message.
///
/// ### Raw Payload Examples
/// #### Subscription Trades Success
/// See docs: <https://www.gate.io/docs/developers/apiv4/ws/en/#api-overview>
/// ```json
/// {
///     "time": 1606292218,
///     "time_ms": 1606292218231,
///     "channel": "spot.trades",
///     "event": "subscribe",
///     "result": {
///         "status": "success"
///     }
/// }
/// ```
///
/// #### Subscription Trades Failure
/// See docs: <https://www.gate.io/docs/developers/apiv4/ws/en/#api-overview>
/// ```json
/// {
///     "time": 1606292218,
///     "time_ms": 1606292218231,
///     "channel": "spot.trades",
///     "event": "subscribe",
///     "error":{
///         "code":2,
///         "message":"unknown currency pair GIBBERISH_USD"
///     },
///     "result": null,
/// }
/// ```
///
/// #### Spot Trade
/// See docs: <https://www.gate.io/docs/developers/apiv4/ws/en/#public-trades-channel>
/// ```json
/// {
///     "time": 1606292218,
///     "time_ms": 1606292218231,
///     "channel": "spot.trades",
///     "event": "update",
///     "result": {
///         "id": 309143071,
///         "create_time": 1606292218,
///         "create_time_ms": "1606292218213.4578",
///         "side": "sell",
///         "currency_pair": "GT_USDT",
///         "amount": "16.4700000000",
///         "price": "0.4705000000"
///     }
/// }
/// ```
///
/// See docs: <https://www.gate.io/docs/developers/apiv4/ws/en/#public-trades-channel>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct GateioMessage<T> {
    pub channel: String,
    pub error: Option<GateioError>,
    #[serde(rename = "result")]
    pub data: T,
}

/// [`Gateio`](super::Gateio) WebSocket error message.
///
/// See docs: <https://www.gate.io/docs/developers/apiv4/ws/en/#public-trades-channel>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct GateioError {
    pub code: u8,
    pub message: String,
}
