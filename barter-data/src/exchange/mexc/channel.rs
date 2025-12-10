//! MEXC channel definitions for WebSocket subscriptions.

use crate::{
    Identifier,
    subscription::{
        Subscription,
        book::{OrderBooksL1, OrderBooksL2},
    },
};

/// MEXC WebSocket channel identifier.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum MexcChannel {
    /// L1 partial depth channel (snapshots of top N levels).
    /// Format: `spot@public.limit.depth.v3.api.pb@{SYMBOL}@{LEVEL}`
    LimitDepth { level: MexcDepthLevel },

    /// L2 aggregated depth channel (incremental updates).
    /// Format: `spot@public.aggre.depth.v3.api.pb@{INTERVAL}@{SYMBOL}`
    AggreDepth { interval: MexcDepthInterval },
}

/// Depth levels for L1 limit depth channel.
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub enum MexcDepthLevel {
    Level5,
    Level10,
    #[default]
    Level20,
}

impl MexcDepthLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            MexcDepthLevel::Level5 => "5",
            MexcDepthLevel::Level10 => "10",
            MexcDepthLevel::Level20 => "20",
        }
    }
}

/// Update intervals for L2 aggregated depth channel.
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub enum MexcDepthInterval {
    Ms10,
    #[default]
    Ms100,
}

impl MexcDepthInterval {
    pub fn as_str(&self) -> &'static str {
        match self {
            MexcDepthInterval::Ms10 => "10ms",
            MexcDepthInterval::Ms100 => "100ms",
        }
    }
}

impl AsRef<str> for MexcChannel {
    fn as_ref(&self) -> &str {
        match self {
            MexcChannel::LimitDepth { .. } => "limit.depth",
            MexcChannel::AggreDepth { .. } => "aggre.depth",
        }
    }
}

impl MexcChannel {
    /// Build the full channel string for subscription.
    pub fn subscription_channel(&self, symbol: &str) -> String {
        match self {
            MexcChannel::LimitDepth { level } => {
                format!(
                    "spot@public.limit.depth.v3.api.pb@{}@{}",
                    symbol.to_uppercase(),
                    level.as_str()
                )
            }
            MexcChannel::AggreDepth { interval } => {
                format!(
                    "spot@public.aggre.depth.v3.api.pb@{}@{}",
                    interval.as_str(),
                    symbol.to_uppercase()
                )
            }
        }
    }
}

impl<Exchange, Instrument> Identifier<MexcChannel> for Subscription<Exchange, Instrument, OrderBooksL1> {
    fn id(&self) -> MexcChannel {
        MexcChannel::LimitDepth {
            level: MexcDepthLevel::Level20,
        }
    }
}

impl<Exchange, Instrument> Identifier<MexcChannel> for Subscription<Exchange, Instrument, OrderBooksL2> {
    fn id(&self) -> MexcChannel {
        MexcChannel::AggreDepth {
            interval: MexcDepthInterval::Ms100,
        }
    }
}


