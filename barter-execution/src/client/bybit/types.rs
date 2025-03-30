use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InstrumentCategory {
    Spot,
    Linear,
    Inverse,
    Option,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum BybitOrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum BybitOrderType {
    Limit,
    Market,
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BybitPositionSide {
    OneWay = 0,
    Long = 1,
    Short = 2,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum ExecutionType {
    Trade,
    AdlTrade,
    Funding,
    BustTrade,
    Delivery,
    Settle,
    BlockTrade,
    MovePosition,
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
