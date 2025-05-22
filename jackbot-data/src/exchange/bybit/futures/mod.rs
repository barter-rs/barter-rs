use self::liquidation::BybitLiquidation;
use super::{Bybit, ExchangeServer};
use crate::{
    ExchangeWsStream, NoInitialSnapshots, exchange::StreamSelector, instrument::InstrumentData,
    subscription::liquidation::Liquidations, transformer::stateless::StatelessTransformer,
};
use jackbot_instrument::exchange::ExchangeId;
use std::fmt::Display;

/// Liquidation types.
pub mod liquidation;

/// L2 order book implementation
pub mod l2;

/// Trade types.
pub mod trade;

/// [`BybitPerpetualsUsd`] WebSocket server base url.
///
/// See docs: <https://bybit-exchange.github.io/docs/v5/ws/connect>
pub const WEBSOCKET_BASE_URL_BYBIT_PERPETUALS_USD: &str = "wss://stream.bybit.com/v5/public/linear";

/// [`Bybit`] perpetual execution.
pub type BybitPerpetualsUsd = Bybit<BybitServerPerpetualsUsd>;

/// [`Bybit`] perpetual [`ExchangeServer`].
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct BybitServerPerpetualsUsd;

impl ExchangeServer for BybitServerPerpetualsUsd {
    const ID: ExchangeId = ExchangeId::BybitPerpetualsUsd;

    fn websocket_url() -> &'static str {
        WEBSOCKET_BASE_URL_BYBIT_PERPETUALS_USD
    }
}

impl<Instrument> StreamSelector<Instrument, Liquidations> for BybitPerpetualsUsd
where
    Instrument: InstrumentData,
{
    type SnapFetcher = NoInitialSnapshots;
    type Stream = ExchangeWsStream<
        StatelessTransformer<Self, Instrument::Key, Liquidations, BybitLiquidation>,
    >;
}

impl Display for BybitPerpetualsUsd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BybitPerpetualsUsd")
    }
}
