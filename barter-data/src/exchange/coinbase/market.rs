use super::Coinbase;
use crate::{instrument::MarketInstrumentData, subscription::Subscription, Identifier};
use barter_instrument::{asset::symbol::Symbol, instrument::Instrument, Keyed};
use serde::{Deserialize, Serialize};
use smol_str::{format_smolstr, SmolStr, StrExt};

/// Type that defines how to translate a Barter [`Subscription`] into a
/// [`Coinbase`] market that can be subscribed to.
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-overview#subscribe>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct CoinbaseMarket(pub SmolStr);

impl<Kind> Identifier<CoinbaseMarket> for Subscription<Coinbase, Instrument, Kind> {
    fn id(&self) -> CoinbaseMarket {
        coinbase_market(&self.instrument.base, &self.instrument.quote)
    }
}

impl<InstrumentKey, Kind> Identifier<CoinbaseMarket>
    for Subscription<Coinbase, Keyed<InstrumentKey, Instrument>, Kind>
{
    fn id(&self) -> CoinbaseMarket {
        coinbase_market(&self.instrument.value.base, &self.instrument.value.quote)
    }
}

impl<Kind> Identifier<CoinbaseMarket> for Subscription<Coinbase, MarketInstrumentData, Kind> {
    fn id(&self) -> CoinbaseMarket {
        CoinbaseMarket(self.instrument.name_exchange.clone())
    }
}

impl AsRef<str> for CoinbaseMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

fn coinbase_market(base: &Symbol, quote: &Symbol) -> CoinbaseMarket {
    CoinbaseMarket(format_smolstr!("{base}-{quote}").to_uppercase_smolstr())
}
