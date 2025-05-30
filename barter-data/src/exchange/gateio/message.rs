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
///
/// #### Spot OrderBooksL1
/// See docs: <https://www.gate.io/docs/developers/apiv4/ws/en/#best-bid-or-ask-price>
/// ```json
/// {
///     "time": 1606292218,
///     "time_ms": 1606292218231,
///     "channel": "spot.book_ticker",
///     "event": "update",
///     "result": {
///         "t": 1606293275123,
///         "u": 48733182,
///         "s": "BTC_USDT",
///         "b": "19177.79",
///         "B": "0.0003341504",
///         "a": "19179.38",
///         "A": "0.09"
///     }
/// }
/// ```
///
/// See docs: <https://www.gate.io/docs/developers/apiv4/ws/en/#best-bid-or-ask-price>
///
/// #### Spot OrderBooksL2
/// See docs: <https://www.gate.io/docs/developers/apiv4/ws/en/#limited-level-full-order-book-snapshot>
/// ```json
/// {
///     "time": 1606292218,
///     "time_ms": 1606292218231,
///     "channel": "spot.order_book",
///     "event": "update",
///     "result": {
///         "t": 1606295412123,
///          "lastUpdateId": 48791820,
///          "s": "BTC_USDT",
///          "l": "5",
///          "bids": [
///              ["19079.55", "0.0195"],
///              ["19079.07", "0.7341"],
///              ["19076.23", "0.00011808"],
///              ["19073.9", "0.105"],
///              ["19068.83", "0.1009"]
///          ],
///          "asks": [
///              ["19080.24", "0.1638"],
///              ["19080.91", "0.1366"],
///              ["19080.92", "0.01"],
///              ["19081.29", "0.01"],
///              ["19083.8", "0.097"]
///          ]
///      }
/// }
/// ```
///
/// See docs: <https://www.gate.io/docs/developers/apiv4/ws/en/#limited-level-full-order-book-snapshot>
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
