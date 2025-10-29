use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::{
    error::UnindexedApiError,
    order::{OrderKind, TimeInForce},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum AccountType {
    Unified,
    Contract,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InstrumentCategory {
    Spot,
    Linear,
    Inverse,
    Option,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BybitOrderTimeInForce {
    #[serde(rename = "GTC")]
    GoodTillCancelled,
    #[serde(rename = "FOK")]
    FillOrKill,
    #[serde(rename = "IOC")]
    ImmediateOrCancelled,
    #[serde(rename = "PostOnly")]
    PostOnly,
}

impl TryFrom<TimeInForce> for BybitOrderTimeInForce {
    type Error = UnindexedApiError;

    fn try_from(value: TimeInForce) -> Result<Self, Self::Error> {
        match value {
            TimeInForce::GoodUntilCancelled { post_only } => match post_only {
                true => Ok(Self::PostOnly),
                false => Ok(Self::GoodTillCancelled),
            },
            TimeInForce::GoodUntilEndOfDay => Err(crate::error::UnindexedApiError::OrderRejected(
                format!("time in force {value} not supported by exchange"),
            )),
            TimeInForce::FillOrKill => Ok(Self::FillOrKill),
            TimeInForce::ImmediateOrCancel => Ok(Self::ImmediateOrCancelled),
        }
    }
}

impl From<BybitOrderTimeInForce> for TimeInForce {
    fn from(value: BybitOrderTimeInForce) -> Self {
        match value {
            BybitOrderTimeInForce::GoodTillCancelled => {
                Self::GoodUntilCancelled { post_only: false }
            }
            BybitOrderTimeInForce::FillOrKill => Self::FillOrKill,
            BybitOrderTimeInForce::ImmediateOrCancelled => Self::ImmediateOrCancel,
            BybitOrderTimeInForce::PostOnly => Self::GoodUntilCancelled { post_only: true },
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum BybitPositionSide {
    OneWay = 0,
    Long = 1,
    Short = 2,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum BybitOrderStatus {
    // Open status
    New,
    PartiallyFilled,
    Untriggered,
    // Closed status
    Rejected,
    PartiallyFilledCanceled,
    Filled,
    Cancelled,
    Triggered,
    Deactivated,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum BybitOrderType {
    Limit,
    Market,
}

impl From<BybitOrderType> for OrderKind {
    fn from(value: BybitOrderType) -> Self {
        match value {
            BybitOrderType::Limit => Self::Limit,
            BybitOrderType::Market => Self::Market,
        }
    }
}
