use crate::{
    Identifier,
    instrument::InstrumentData,
    subscription::{
        Subscription,
        book::{OrderBooksL1, OrderBooksL2},
        trade::PublicTrades,
    },
};
use barter_instrument::instrument::market_data::kind::MarketDataInstrumentKind;
use serde::Serialize;

use super::Gateio;

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

    /// Gateio [`MarketDataInstrumentKind::Spot`] real-time trades channel.
    ///
    /// See docs: <https://www.gate.io/docs/developers/apiv4/ws/en/#best-bid-or-ask-price>
    pub const ORDER_BOOK_L1: Self = Self("spot.book_ticker");

    /// Gateio [`MarketDataInstrumentKind::Spot`] real-time trades channel.
    ///
    /// See docs: <https://www.gate.io/docs/developers/apiv4/ws/en/#changed-order-book-levels>
    pub const ORDER_BOOK_L2: Self = Self("spot.order_book_update");
}

impl<Server, Instrument> Identifier<GateioChannel>
    for Subscription<Gateio<Server>, Instrument, PublicTrades>
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

impl<Server, Instrument> Identifier<GateioChannel>
    for Subscription<Gateio<Server>, Instrument, OrderBooksL2>
{
    fn id(&self) -> GateioChannel {
        GateioChannel::ORDER_BOOK_L2
    }
}

impl<Server, Instrument> Identifier<GateioChannel>
    for Subscription<Gateio<Server>, Instrument, OrderBooksL1>
{
    fn id(&self) -> GateioChannel {
        GateioChannel::ORDER_BOOK_L1
    }
}

impl AsRef<str> for GateioChannel {
    fn as_ref(&self) -> &str {
        self.0
    }
}
