use super::Gateio;
use crate::{
    instrument::{KeyedInstrument, MarketInstrumentData},
    subscription::Subscription,
    Identifier,
};
use barter_integration::model::instrument::{
    kind::{InstrumentKind, OptionKind},
    Instrument,
};
use chrono::{
    format::{DelayedFormat, StrftimeItems},
    DateTime, Utc,
};
use serde::{Deserialize, Serialize};

/// Type that defines how to translate a Barter [`Subscription`] into a
/// [`Gateio`](super::Gateio) market that can be subscribed to.
///
/// See docs: <https://www.okx.com/docs-v5/en/#websocket-api-public-channel>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct GateioMarket(pub String);

impl<Server, Kind> Identifier<GateioMarket> for Subscription<Gateio<Server>, Instrument, Kind> {
    fn id(&self) -> GateioMarket {
        gateio_market(&self.instrument)
    }
}

impl<Server, Kind> Identifier<GateioMarket>
    for Subscription<Gateio<Server>, KeyedInstrument, Kind>
{
    fn id(&self) -> GateioMarket {
        gateio_market(&self.instrument.data)
    }
}

impl<Server, Kind> Identifier<GateioMarket>
    for Subscription<Gateio<Server>, MarketInstrumentData, Kind>
{
    fn id(&self) -> GateioMarket {
        GateioMarket(self.instrument.name_exchange.clone())
    }
}

impl AsRef<str> for GateioMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

fn gateio_market(instrument: &Instrument) -> GateioMarket {
    use InstrumentKind::*;
    let Instrument { base, quote, kind } = instrument;

    GateioMarket(
        match kind {
            Spot | Perpetual => format!("{base}_{quote}"),
            Future(future) => {
                format!("{base}_{quote}_QUARTERLY_{}", format_expiry(future.expiry))
            }
            Option(option) => format!(
                "{base}_{quote}-{}-{}-{}",
                format_expiry(option.expiry),
                option.strike,
                match option.kind {
                    OptionKind::Call => "C",
                    OptionKind::Put => "P",
                },
            ),
        }
        .to_uppercase(),
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
