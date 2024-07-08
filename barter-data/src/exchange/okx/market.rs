use super::Okx;
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
/// [`Okx`] market that can be subscribed to.
///
/// See docs: <https://www.okx.com/docs-v5/en/#websocket-api-public-channel>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct OkxMarket(pub String);

impl<Kind> Identifier<OkxMarket> for Subscription<Okx, Instrument, Kind> {
    fn id(&self) -> OkxMarket {
        okx_market(&self.instrument)
    }
}

impl<Kind> Identifier<OkxMarket> for Subscription<Okx, KeyedInstrument, Kind> {
    fn id(&self) -> OkxMarket {
        okx_market(&self.instrument.data)
    }
}

impl<Kind> Identifier<OkxMarket> for Subscription<Okx, MarketInstrumentData, Kind> {
    fn id(&self) -> OkxMarket {
        OkxMarket(self.instrument.name_exchange.clone())
    }
}

impl AsRef<str> for OkxMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

fn okx_market(instrument: &Instrument) -> OkxMarket {
    use InstrumentKind::*;
    let Instrument { base, quote, kind } = instrument;

    OkxMarket(match kind {
        Spot => format!("{base}-{quote}").to_uppercase(),
        Future(future) => format!("{base}-{quote}-{}", format_expiry(future.expiry)).to_uppercase(),
        Perpetual => format!("{base}-{quote}-SWAP").to_uppercase(),
        Option(option) => format!(
            "{base}-{quote}-{}-{}-{}",
            format_expiry(option.expiry),
            option.strike,
            match option.kind {
                OptionKind::Call => "C",
                OptionKind::Put => "P",
            },
        )
        .to_uppercase(),
    })
}

/// Format the expiry DateTime<Utc> to be Okx API compatible.
///
/// eg/ "230526" (26th of May 2023)
///
/// See docs: <https://www.okx.com/docs-v5/en/#rest-api-public-data-get-instruments>
fn format_expiry<'a>(expiry: DateTime<Utc>) -> DelayedFormat<StrftimeItems<'a>> {
    expiry.date_naive().format("%g%m%d")
}
