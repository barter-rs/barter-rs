use super::Bitfinex;
use crate::{
    instrument::{KeyedInstrument, MarketInstrumentData},
    subscription::Subscription,
    Identifier,
};
use barter_integration::model::instrument::{symbol::Symbol, Instrument};
use serde::{Deserialize, Serialize};

/// Type that defines how to translate a Barter [`Subscription`] into a
/// [`Bitfinex`] market that can be subscribed to.
///
/// See docs: <https://docs.bitfinex.com/docs/ws-public>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct BitfinexMarket(pub String);

impl<Kind> Identifier<BitfinexMarket> for Subscription<Bitfinex, Instrument, Kind> {
    fn id(&self) -> BitfinexMarket {
        bitfinex_market(&self.instrument.base, &self.instrument.quote)
    }
}

impl<Kind> Identifier<BitfinexMarket> for Subscription<Bitfinex, KeyedInstrument, Kind> {
    fn id(&self) -> BitfinexMarket {
        bitfinex_market(&self.instrument.data.base, &self.instrument.data.quote)
    }
}

impl<Kind> Identifier<BitfinexMarket> for Subscription<Bitfinex, MarketInstrumentData, Kind> {
    fn id(&self) -> BitfinexMarket {
        BitfinexMarket(self.instrument.name_exchange.clone())
    }
}

impl AsRef<str> for BitfinexMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

fn bitfinex_market(base: &Symbol, quote: &Symbol) -> BitfinexMarket {
    BitfinexMarket(format!(
        "t{}{}",
        base.to_string().to_uppercase(),
        quote.to_string().to_uppercase()
    ))
}
