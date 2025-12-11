//! MEXC market identifier.

use super::MexcSpot;
use crate::{Identifier, instrument::MarketInstrumentData, subscription::Subscription};
use barter_instrument::{
    Keyed, asset::name::AssetNameInternal, instrument::market_data::MarketDataInstrument,
};
use smol_str::{SmolStr, StrExt, format_smolstr};

/// MEXC market identifier.
///
/// Format: Base asset + Quote asset (e.g., "BTCUSDT").
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct MexcMarket(pub SmolStr);

impl AsRef<str> for MexcMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl<Kind> Identifier<MexcMarket> for Subscription<MexcSpot, MarketDataInstrument, Kind> {
    fn id(&self) -> MexcMarket {
        mexc_market(&self.instrument.base, &self.instrument.quote)
    }
}

impl<InstrumentKey, Kind> Identifier<MexcMarket>
    for Subscription<MexcSpot, Keyed<InstrumentKey, MarketDataInstrument>, Kind>
{
    fn id(&self) -> MexcMarket {
        mexc_market(
            &self.instrument.as_ref().base,
            &self.instrument.as_ref().quote,
        )
    }
}

impl<InstrumentKey, Kind> Identifier<MexcMarket>
    for Subscription<MexcSpot, MarketInstrumentData<InstrumentKey>, Kind>
{
    fn id(&self) -> MexcMarket {
        MexcMarket(self.instrument.name_exchange.name().clone())
    }
}

/// Build a [`MexcMarket`] identifier from base and quote assets.
pub fn mexc_market(base: &AssetNameInternal, quote: &AssetNameInternal) -> MexcMarket {
    // MEXC uses uppercase base+quote format
    MexcMarket(format_smolstr!("{base}{quote}").to_uppercase_smolstr())
}

/// Extract the symbol from a MEXC channel string.
///
/// Channel formats:
/// - L1: `spot@public.limit.depth.v3.api.pb@{SYMBOL}@{LEVEL}`
/// - L2: `spot@public.aggre.depth.v3.api.pb@{INTERVAL}@{SYMBOL}`
pub fn extract_symbol_from_channel(channel: &str) -> Option<&str> {
    let parts: Vec<&str> = channel.split('@').collect();
    if parts.len() < 4 {
        return None;
    }

    // Check if it's limit.depth (L1) or aggre.depth (L2)
    if channel.contains("limit.depth") {
        // Format: spot@public.limit.depth.v3.api.pb@{SYMBOL}@{LEVEL}
        Some(parts[2])
    } else if channel.contains("aggre.depth") {
        // Format: spot@public.aggre.depth.v3.api.pb@{INTERVAL}@{SYMBOL}
        Some(parts[3])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_symbol_l1() {
        let channel = "spot@public.limit.depth.v3.api.pb@BTCUSDT@20";
        assert_eq!(extract_symbol_from_channel(channel), Some("BTCUSDT"));
    }

    #[test]
    fn test_extract_symbol_l2() {
        let channel = "spot@public.aggre.depth.v3.api.pb@100ms@ETHUSDT";
        assert_eq!(extract_symbol_from_channel(channel), Some("ETHUSDT"));
    }
}
