use super::Ibkr;
use crate::{subscription::Subscription, Identifier};
use barter_instrument::instrument::{
    kind::option::OptionKind,
    market_data::{kind::MarketDataInstrumentKind, MarketDataInstrument},
};
use chrono::{
    format::{DelayedFormat, StrftimeItems},
    DateTime, Utc,
};
use serde::{Deserialize, Serialize};

/// Type that defines how to translate a Barter [`Subscription`] into a
/// [`Ibkr`](super::Ibkr) market that can be subscribed to.
///
/// See docs:
///
/// https://www.interactivebrokers.com/campus/ibkr-api-page/cpapi-v1/#ws-subscribing-topics
///
/// https://www.interactivebrokers.com/campus/ibkr-api-page/cpapi-v1/#ws-sub-watchlist-data
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct IbkrMarket {
    instrument: MarketDataInstrument,
    pub contract_id: String,
    pub fields: String,
}

// TODO: refactor like market.rs in other exchanges
// ex: ibkr_market()
impl<Kind> Identifier<IbkrMarket> for Subscription<Ibkr, MarketDataInstrument, Kind> {
    fn id(&self) -> IbkrMarket {
        use MarketDataInstrumentKind::*;
        let MarketDataInstrument { base, quote, kind } = &self.instrument;

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
            fields: r#""31","84","88","86","85""#.to_string(),
        }
    }
}

impl AsRef<str> for IbkrMarket {
    fn as_ref(&self) -> &str {
        &self.contract_id
    }
}

/// TODO: is this still required or remnants of Okx? i.e. do we need YYYY?
/// Format the expiry DateTime<Utc> to be Ibkr API compatible.
///
/// eg/ "230526" (26th of May 2023)
///
/// See docs:
fn format_expiry<'a>(expiry: DateTime<Utc>) -> DelayedFormat<StrftimeItems<'a>> {
    expiry.date_naive().format("%g%m%d")
}
