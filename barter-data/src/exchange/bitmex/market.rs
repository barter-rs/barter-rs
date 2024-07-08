use crate::{
    exchange::bitmex::Bitmex,
    instrument::{KeyedInstrument, MarketInstrumentData},
    subscription::Subscription,
    Identifier,
};
use barter_integration::model::instrument::{symbol::Symbol, Instrument};
use serde::{Deserialize, Serialize};

/// Type that defines how to translate a Barter [`Subscription`] into a [`Bitmex`]
/// market that can be subscribed to.
///
/// See docs: <https://www.bitmex.com/app/wsAPI>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct BitmexMarket(pub String);

impl<Kind> Identifier<BitmexMarket> for Subscription<Bitmex, Instrument, Kind> {
    fn id(&self) -> BitmexMarket {
        bitmex_market(&self.instrument.base, &self.instrument.quote)
    }
}

impl<Kind> Identifier<BitmexMarket> for Subscription<Bitmex, KeyedInstrument, Kind> {
    fn id(&self) -> BitmexMarket {
        bitmex_market(&self.instrument.data.base, &self.instrument.data.quote)
    }
}

impl<Kind> Identifier<BitmexMarket> for Subscription<Bitmex, MarketInstrumentData, Kind> {
    fn id(&self) -> BitmexMarket {
        BitmexMarket(self.instrument.name_exchange.clone())
    }
}

impl AsRef<str> for BitmexMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

fn bitmex_market(base: &Symbol, quote: &Symbol) -> BitmexMarket {
    // Notes:
    // - Must be uppercase since Bitmex sends message with uppercase MARKET (eg/ XBTUSD).
    BitmexMarket(format!("{base}{quote}").to_uppercase())
}
