use super::Gateio;
use crate::{Identifier, instrument::MarketInstrumentData, subscription::Subscription};
use barter_instrument::{
    Keyed,
    instrument::{
        kind::option::OptionKind,
        market_data::{MarketDataInstrument, kind::MarketDataInstrumentKind::*},
    },
};
use chrono::{
    DateTime, Utc,
    format::{DelayedFormat, StrftimeItems},
};
use serde::{Deserialize, Serialize};
use smol_str::{SmolStr, StrExt, format_smolstr};

/// Type that defines how to translate a Barter [`Subscription`] into a
/// [`Gateio`] market that can be subscribed to.
///
/// See docs: <https://www.okx.com/docs-v5/en/#websocket-api-public-channel>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct GateioMarket(pub SmolStr);

impl<Server, Kind> Identifier<GateioMarket>
    for Subscription<Gateio<Server>, MarketDataInstrument, Kind>
{
    fn id(&self) -> GateioMarket {
        gateio_market(&self.instrument)
    }
}

impl<Server, InstrumentKey, Kind> Identifier<GateioMarket>
    for Subscription<Gateio<Server>, Keyed<InstrumentKey, MarketDataInstrument>, Kind>
{
    fn id(&self) -> GateioMarket {
        gateio_market(&self.instrument.value)
    }
}

impl<Server, InstrumentKey, Kind> Identifier<GateioMarket>
    for Subscription<Gateio<Server>, MarketInstrumentData<InstrumentKey>, Kind>
{
    fn id(&self) -> GateioMarket {
        GateioMarket(self.instrument.name_exchange.name().clone())
    }
}

impl AsRef<str> for GateioMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

fn gateio_market(instrument: &MarketDataInstrument) -> GateioMarket {
    let MarketDataInstrument { base, quote, kind } = instrument;

    GateioMarket(
        match kind {
            Spot | Perpetual => format_smolstr!("{base}_{quote}"),
            Future(contract) => {
                format_smolstr!(
                    "{base}_{quote}_QUARTERLY_{}",
                    format_expiry(contract.expiry)
                )
            }
            Option(contract) => format_smolstr!(
                "{base}_{quote}-{}-{}-{}",
                format_expiry(contract.expiry),
                contract.strike,
                match contract.kind {
                    OptionKind::Call => "C",
                    OptionKind::Put => "P",
                },
            ),
        }
        .to_uppercase_smolstr(),
    )
}

/// Format the expiry DateTime<Utc> to be Gateio API compatible.
///
/// eg/ "20241231" (31st of December 2024)
///
/// See docs: <https://www.gate.io/docs/developers/options/ws/en/#public-contract-trades-channel>
fn format_expiry<'a>(expiry: DateTime<Utc>) -> DelayedFormat<StrftimeItems<'a>> {
    expiry.date_naive().format("%Y%m%d")
}
