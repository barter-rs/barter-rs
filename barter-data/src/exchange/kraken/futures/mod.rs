use crate::exchange::ExchangeServer;
use barter_instrument::exchange::ExchangeId;

/// [`KrakenFuturesUsd`](super::KrakenFuturesUsd) execution server.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct KrakenServerFuturesUsd;

impl ExchangeServer for KrakenServerFuturesUsd {
    const ID: ExchangeId = ExchangeId::KrakenFuturesUsd;

    fn websocket_url() -> &'static str {
        "wss://futures.kraken.com/ws/v1"
    }
}
