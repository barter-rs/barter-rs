use crate::{
    Identifier,
    instrument::InstrumentData,
    subscription::{Subscription, trade::PublicTrades},
};
use barter_instrument::instrument::market_data::kind::MarketDataInstrumentKind;
use serde::Serialize;

/// Type that defines how to translate a Barter [`Subscription`] into a
/// [`Gateio`](super::Gateio) channel to be subscribed to.
///
/// See docs: <https://www.okx.com/docs-v5/en/#websocket-api-public-channel>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub struct GateioChannel(pub &'static str);

impl GateioChannel {
    /// Gateio [`MarketDataInstrumentKind::Spot`] real-time trades channel.
    ///
    /// See docs: <https://www.gate.io/docs/developers/apiv4/ws/en/#public-trades-channel>
    pub const SPOT_TRADES: Self = Self("spot.trades");

    /// Gateio [`MarketDataInstrumentKind::Future`] & [`MarketDataInstrumentKind::Perpetual`] real-time trades channel.
    ///
    /// See docs: <https://www.gate.io/docs/developers/futures/ws/en/#trades-subscription>
    /// See docs: <https://www.gate.io/docs/developers/delivery/ws/en/#trades-subscription>
    pub const FUTURE_TRADES: Self = Self("futures.trades");

    /// Gateio [`MarketDataInstrumentKind::Option`] real-time trades channel.
    ///
    /// See docs: <https://www.gate.io/docs/developers/options/ws/en/#public-contract-trades-channel>
    pub const OPTION_TRADES: Self = Self("options.trades");
}

impl<GateioExchange, Instrument> Identifier<GateioChannel>
    for Subscription<GateioExchange, Instrument, PublicTrades>
where
    Instrument: InstrumentData,
{
    fn id(&self) -> GateioChannel {
        match self.instrument.kind() {
            MarketDataInstrumentKind::Spot => GateioChannel::SPOT_TRADES,
            MarketDataInstrumentKind::Future { .. } | MarketDataInstrumentKind::Perpetual => {
                GateioChannel::FUTURE_TRADES
            }
            MarketDataInstrumentKind::Option { .. } => GateioChannel::OPTION_TRADES,
        }
    }
}

impl AsRef<str> for GateioChannel {
    fn as_ref(&self) -> &str {
        self.0
    }
}
