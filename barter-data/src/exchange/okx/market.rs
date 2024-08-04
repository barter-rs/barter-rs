use super::Okx;
use crate::{instrument::MarketInstrumentData, subscription::Subscription, Identifier};
use barter_instrument::{
    instrument::{
        kind::option::OptionKind,
        market_data::{kind::MarketDataInstrumentKind::*, MarketDataInstrument},
    },
    Keyed,
};
use chrono::{
    format::{DelayedFormat, StrftimeItems},
    DateTime, Utc,
};
use serde::{Deserialize, Serialize};
use smol_str::{format_smolstr, SmolStr, StrExt};

/// Type that defines how to translate a Barter [`Subscription`] into a
/// [`Okx`] market that can be subscribed to.
///
/// See docs: <https://www.okx.com/docs-v5/en/#websocket-api-public-channel>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct OkxMarket(pub SmolStr);

impl<Kind> Identifier<OkxMarket> for Subscription<Okx, MarketDataInstrument, Kind> {
    fn id(&self) -> OkxMarket {
        okx_market(&self.instrument)
    }
}

impl<InstrumentKey, Kind> Identifier<OkxMarket>
    for Subscription<Okx, Keyed<InstrumentKey, MarketDataInstrument>, Kind>
{
    fn id(&self) -> OkxMarket {
        okx_market(&self.instrument.value)
    }
}

impl<InstrumentKey, Kind> Identifier<OkxMarket>
    for Subscription<Okx, MarketInstrumentData<InstrumentKey>, Kind>
{
    fn id(&self) -> OkxMarket {
        OkxMarket(self.instrument.name_exchange.name().clone())
    }
}

impl AsRef<str> for OkxMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

fn okx_market(instrument: &MarketDataInstrument) -> OkxMarket {
    let MarketDataInstrument { base, quote, kind } = instrument;

    OkxMarket(match kind {
        Spot => format_smolstr!("{base}-{quote}").to_uppercase_smolstr(),
        Future(future) => format_smolstr!("{base}-{quote}-{}", format_expiry(future.expiry))
            .to_uppercase_smolstr(),
        Perpetual => format_smolstr!("{base}-{quote}-SWAP").to_uppercase_smolstr(),
        Option(option) => format_smolstr!(
            "{base}-{quote}-{}-{}-{}",
            format_expiry(option.expiry),
            option.strike,
            match option.kind {
                OptionKind::Call => "C",
                OptionKind::Put => "P",
            },
        )
        .to_uppercase_smolstr(),
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
