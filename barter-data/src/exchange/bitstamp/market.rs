use crate::{
    Identifier, exchange::bitstamp::BitstampSpot, instrument::MarketInstrumentData,
    subscription::Subscription,
};
use barter_instrument::{
    Keyed, asset::name::AssetNameInternal, instrument::market_data::MarketDataInstrument,
};
use serde::{Deserialize, Serialize};
use smol_str::{SmolStr, format_smolstr};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct BitstampMarket(pub SmolStr);

impl<Kind> Identifier<BitstampMarket> for Subscription<BitstampSpot, MarketDataInstrument, Kind> {
    fn id(&self) -> BitstampMarket {
        bitstamp_market(&self.instrument.base, &self.instrument.quote)
    }
}

impl<InstrumentKey, Kind> Identifier<BitstampMarket>
    for Subscription<BitstampSpot, Keyed<InstrumentKey, MarketDataInstrument>, Kind>
{
    fn id(&self) -> BitstampMarket {
        bitstamp_market(
            &self.instrument.as_ref().base,
            &self.instrument.as_ref().quote,
        )
    }
}

impl<InstrumentKey, Kind> Identifier<BitstampMarket>
    for Subscription<BitstampSpot, MarketInstrumentData<InstrumentKey>, Kind>
{
    fn id(&self) -> BitstampMarket {
        BitstampMarket(self.instrument.name_exchange.name().clone())
    }
}

impl AsRef<str> for BitstampMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

pub(in crate::exchange::bitstamp) fn bitstamp_market(
    base: &AssetNameInternal,
    quote: &AssetNameInternal,
) -> BitstampMarket {
    BitstampMarket(format_smolstr!("{base}{quote}"))
}
