use super::Ibkr;
use crate::{subscription::Subscription, Identifier};
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
/// [`Ibkr`](super::Ibkr) market that can be subscribed to.
///
/// See docs: <https://www.okx.com/docs-v5/en/#websocket-api-public-channel>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct IbkrMarket {
    instrument: Instrument,
    pub contract_id: String,
    pub fields: String,
}

impl<Kind> Identifier<IbkrMarket> for Subscription<Ibkr, Instrument, Kind> {
    fn id(&self) -> IbkrMarket {
        use InstrumentKind::*;
        let Instrument { base, quote, kind } = &self.instrument;

        IbkrMarket {
            instrument: self.instrument.clone(),
            contract_id: match kind {
                Spot => 265598.to_string(), // AAPL - TODO: implement lookup on self.instrument.base
                Future(future) => {
                    format!("{base}-{quote}-{}", format_expiry(future.expiry)).to_uppercase()
                }
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
            },
            fields: r#""31","84","88","86","85""#.to_string()
        }
    }
}

impl AsRef<str> for IbkrMarket {
    fn as_ref(&self) -> &str {
        &self.contract_id
    }
}

/// Format the expiry DateTime<Utc> to be Ibkr API compatible.
///
/// eg/ "230526" (26th of May 2023)
///
/// See docs: <https://www.okx.com/docs-v5/en/#rest-api-public-data-get-instruments>
fn format_expiry<'a>(expiry: DateTime<Utc>) -> DelayedFormat<StrftimeItems<'a>> {
    expiry.date_naive().format("%g%m%d")
}
