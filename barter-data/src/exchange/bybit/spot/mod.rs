use super::{Bybit, ExchangeServer};
use crate::exchange::ExchangeId;

/// [`BybitSpot`] WebSocket server base url.
///
/// See docs: <https://bybit-exchange.github.io/docs/v5/ws/connect>
pub const WEBSOCKET_BASE_URL_BYBIT_SPOT: &str = "wss://stream.bybit.com/v5/public/spot";

/// [`Bybit`] spot exchange.
pub type BybitSpot = Bybit<BybitServerSpot>;

/// [`Bybit`] spot [`ExchangeServer`].
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct BybitServerSpot;

impl ExchangeServer for BybitServerSpot {
    const ID: ExchangeId = ExchangeId::BybitSpot;

    fn websocket_url() -> &'static str {
        WEBSOCKET_BASE_URL_BYBIT_SPOT
    }
}
