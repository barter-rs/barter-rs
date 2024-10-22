use super::Kraken;
use crate::{instrument::MarketInstrumentData, subscription::Subscription, Identifier};
use barter_instrument::{asset::symbol::Symbol, instrument::Instrument, Keyed};
use serde::{Deserialize, Serialize};
use smol_str::{format_smolstr, SmolStr, StrExt};

/// Type that defines how to translate a Barter [`Subscription`] into a
/// [`Kraken`] market that can be subscribed to.
///
/// See docs: <https://docs.kraken.com/websockets/#message-subscribe>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct KrakenMarket(pub SmolStr);

impl<Kind> Identifier<KrakenMarket> for Subscription<Kraken, Instrument, Kind> {
    fn id(&self) -> KrakenMarket {
        kraken_market(&self.instrument.base, &self.instrument.quote)
    }
}

impl<InstrumentKey, Kind> Identifier<KrakenMarket>
    for Subscription<Kraken, Keyed<InstrumentKey, Instrument>, Kind>
{
    fn id(&self) -> KrakenMarket {
        kraken_market(&self.instrument.value.base, &self.instrument.value.quote)
    }
}

impl<Kind> Identifier<KrakenMarket> for Subscription<Kraken, MarketInstrumentData, Kind> {
    fn id(&self) -> KrakenMarket {
        KrakenMarket(self.instrument.name_exchange.clone())
    }
}

impl AsRef<str> for KrakenMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

fn kraken_market(base: &Symbol, quote: &Symbol) -> KrakenMarket {
    KrakenMarket(format_smolstr!("{base}/{quote}").to_lowercase_smolstr())
}
