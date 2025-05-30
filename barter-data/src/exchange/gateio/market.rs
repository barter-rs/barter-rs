use super::Gateio;
use crate::{
    Identifier,
    instrument::MarketInstrumentData,
    subscription::{Subscription, SubscriptionKind},
};
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
/// See docs: <https://www.gate.io/docs/developers/apiv4/ws/en/#spot-websocket-v4>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct GateioMarket(pub Vec<SmolStr>);

impl<Server, Kind> Identifier<GateioMarket>
    for Subscription<Gateio<Server>, MarketDataInstrument, Kind>
where
    Kind: SubscriptionKind,
{
    fn id(&self) -> GateioMarket {
        gateio_market(&self.instrument, self.kind.as_str() == "l2")
    }
}

impl<Server, InstrumentKey, Kind> Identifier<GateioMarket>
    for Subscription<Gateio<Server>, Keyed<InstrumentKey, MarketDataInstrument>, Kind>
where
    Kind: SubscriptionKind,
{
    fn id(&self) -> GateioMarket {
        gateio_market(&self.instrument.value, self.kind.as_str() == "l2")
    }
}

impl<Server, InstrumentKey, Kind> Identifier<GateioMarket>
    for Subscription<Gateio<Server>, MarketInstrumentData<InstrumentKey>, Kind>
where
    Kind: SubscriptionKind,
{
    fn id(&self) -> GateioMarket {
        if self.kind.as_str() == "l2" {
            GateioMarket(vec![
                self.instrument.name_exchange.name().clone(),
                format_smolstr!("100ms"),
            ])
        } else {
            GateioMarket(vec![self.instrument.name_exchange.name().clone()])
        }
    }
}

impl GateioMarket {
    pub fn as_str_vec(&self) -> Vec<&str> {
        self.0.iter().map(|v| v.as_str()).collect()
    }
}

impl AsRef<str> for GateioMarket {
    fn as_ref(&self) -> &str {
        self.0[0].as_str()
    }
}

fn gateio_market(instrument: &MarketDataInstrument, l2: bool) -> GateioMarket {
    let MarketDataInstrument { base, quote, kind } = instrument;

    let mut smol_strs = vec![
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
    ];
    if l2 {
        smol_strs.push(format_smolstr!("100ms"));
    }
    GateioMarket(smol_strs)
}

/// Format the expiry DateTime<Utc> to be Gateio API compatible.
///
/// eg/ "20241231" (31st of December 2024)
///
/// See docs: <https://www.gate.io/docs/developers/options/ws/en/#public-contract-trades-channel>
fn format_expiry<'a>(expiry: DateTime<Utc>) -> DelayedFormat<StrftimeItems<'a>> {
    expiry.date_naive().format("%Y%m%d")
}
