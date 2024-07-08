use crate::{
    exchange::bybit::Bybit,
    instrument::{KeyedInstrument, MarketInstrumentData},
    subscription::Subscription,
    Identifier,
};
use barter_integration::model::instrument::{symbol::Symbol, Instrument};
use serde::{Deserialize, Serialize};

/// Type that defines how to translate a Barter [`Subscription`] into a [`Bybit`]
/// market that can be subscribed to.
///
/// See docs: <https://bybit-exchange.github.io/docs/v5/ws/connect>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct BybitMarket(pub String);

impl<Server, Kind> Identifier<BybitMarket> for Subscription<Bybit<Server>, Instrument, Kind> {
    fn id(&self) -> BybitMarket {
        bybit_market(&self.instrument.base, &self.instrument.quote)
    }
}

impl<Server, Kind> Identifier<BybitMarket> for Subscription<Bybit<Server>, KeyedInstrument, Kind> {
    fn id(&self) -> BybitMarket {
        bybit_market(&self.instrument.data.base, &self.instrument.data.quote)
    }
}

impl<Server, Kind> Identifier<BybitMarket>
    for Subscription<Bybit<Server>, MarketInstrumentData, Kind>
{
    fn id(&self) -> BybitMarket {
        BybitMarket(self.instrument.name_exchange.clone())
    }
}

impl AsRef<str> for BybitMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

fn bybit_market(base: &Symbol, quote: &Symbol) -> BybitMarket {
    // Notes:
    // - Must be uppercase since Bybit sends message with uppercase MARKET (eg/ BTCUSDT).
    BybitMarket(format!("{base}{quote}").to_uppercase())
}
