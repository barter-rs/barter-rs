use crate::{
    instrument::InstrumentData,
    subscription::{trade::PublicTrades, Subscription},
    Identifier,
};
use barter_integration::model::instrument::kind::InstrumentKind;
use serde::Serialize;

/// Type that defines how to translate a Barter [`Subscription`] into a
/// [`Gateio`](super::Gateio) channel to be subscribed to.
///
/// See docs: <https://www.okx.com/docs-v5/en/#websocket-api-public-channel>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub struct GateioChannel(pub &'static str);

impl GateioChannel {
    /// Gateio [`InstrumentKind::Spot`] real-time trades channel.
    ///
    /// See docs: <https://www.gate.io/docs/developers/apiv4/ws/en/#public-trades-channel>
    pub const SPOT_TRADES: Self = Self("spot.trades");

    /// Gateio [`InstrumentKind::Future`] & [`InstrumentKind::Perpetual`] real-time trades channel.
    ///
    /// See docs: <https://www.gate.io/docs/developers/futures/ws/en/#trades-subscription>
    /// See docs: <https://www.gate.io/docs/developers/delivery/ws/en/#trades-subscription>
    pub const FUTURE_TRADES: Self = Self("futures.trades");

    /// Gateio [`InstrumentKind::Option`] real-time trades channel.
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
            InstrumentKind::Spot => GateioChannel::SPOT_TRADES,
            InstrumentKind::Future(_) | InstrumentKind::Perpetual => GateioChannel::FUTURE_TRADES,
            InstrumentKind::Option(_) => GateioChannel::OPTION_TRADES,
        }
    }
}

impl AsRef<str> for GateioChannel {
    fn as_ref(&self) -> &str {
        self.0
    }
}
