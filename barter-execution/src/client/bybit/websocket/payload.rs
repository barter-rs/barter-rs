use barter_instrument::Side;
use barter_instrument::asset::QuoteAsset;
use barter_instrument::asset::name::AssetNameExchange;
use barter_instrument::exchange::ExchangeId;
use barter_instrument::instrument::name::InstrumentNameExchange;
use barter_integration::de::de_u64_epoch_ms_as_datetime_utc;
use barter_integration::error::SocketError;
use barter_integration::snapshot::Snapshot;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use serde::Deserialize;
use serde_json::value::RawValue;
use serde_with::{DefaultOnError, DisplayFromStr, serde_as};

use crate::client::bybit::types::{
    BybitOrderStatus, BybitOrderTimeInForce, BybitOrderType, BybitPositionSide, InstrumentCategory,
};
use crate::order::id::{ClientOrderId, OrderId, StrategyId};
use crate::order::request::OrderResponseCancel;
use crate::order::state::{ActiveOrderState, Cancelled, Open, OrderState};
use crate::order::{Order, OrderKey};
use crate::trade::{AssetFees, Trade, TradeId};
use crate::{AccountEvent, AccountEventKind, UnindexedAccountEvent};

#[derive(Debug, Deserialize)]
pub struct BybitPayload {
    #[serde(alias = "topic")]
    pub topic: BybitPayloadTopic,

    #[serde(
        alias = "creationTime",
        deserialize_with = "de_u64_epoch_ms_as_datetime_utc"
    )]
    pub timestamp: DateTime<Utc>,

    #[serde(rename = "data")]
    pub data: Box<RawValue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BybitPayloadTopic {
    Order,
    Execution,
}

#[serde_as]
#[derive(Clone, PartialEq, Debug, Deserialize)]
pub struct OrderUpdateData {
    #[serde(rename = "category")]
    pub category: InstrumentCategory,

    #[serde(rename = "symbol")]
    pub symbol: InstrumentNameExchange,

    #[serde(rename = "orderId")]
    pub exchange_order_id: OrderId,

    #[serde_as(as = "DefaultOnError<Option<DisplayFromStr>>")]
    #[serde(rename = "orderLinkId")]
    pub client_order_id: Option<String>,

    #[serde(rename = "side")]
    pub side: Side,

    #[serde(rename = "orderType")]
    pub order_type: BybitOrderType,

    #[serde(rename = "timeInForce")]
    pub time_in_force: BybitOrderTimeInForce,

    #[serde(rename = "positionIdx")]
    pub position_side: Option<BybitPositionSide>,

    #[serde(rename = "orderStatus")]
    pub status: BybitOrderStatus,

    #[serde(rename = "rejectReason")]
    pub rejection_reason: String,

    #[serde(rename = "cancelType")]
    pub cancel_type: String,

    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "qty")]
    pub original_quantity: f64,

    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "price")]
    pub original_price: f64,

    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "cumExecQty")]
    pub cumulative_executed_quantity: f64,

    #[serde_as(as = "DefaultOnError<Option<DisplayFromStr>>")]
    #[serde(rename = "avgPrice")]
    pub average_price: Option<f64>,
}

impl TryFrom<(ExchangeId, OrderUpdateData, DateTime<Utc>)> for UnindexedAccountEvent {
    type Error = SocketError;

    fn try_from(
        (exchange, order, time_exchange): (ExchangeId, OrderUpdateData, DateTime<Utc>),
    ) -> Result<Self, Self::Error> {
        let key = OrderKey {
            exchange,
            instrument: order.symbol,
            strategy: StrategyId::unknown(),
            cid: ClientOrderId::new(order.client_order_id.unwrap()),
        };

        let kind = match order.status {
            BybitOrderStatus::New
            | BybitOrderStatus::PartiallyFilled
            | BybitOrderStatus::Untriggered => AccountEventKind::OrderSnapshot(Snapshot(Order {
                key,
                side: order.side,
                price: Decimal::from_f64(order.original_price).unwrap(),
                quantity: Decimal::from_f64(order.original_quantity).unwrap(),
                kind: order.order_type.into(),
                time_in_force: order.time_in_force.into(),
                state: OrderState::<AssetNameExchange, _>::active(ActiveOrderState::Open(Open {
                    id: order.exchange_order_id,
                    time_exchange,
                    filled_quantity: Decimal::from_f64(order.cumulative_executed_quantity).unwrap(),
                })),
            })),
            BybitOrderStatus::Rejected
            | BybitOrderStatus::PartiallyFilledCanceled
            | BybitOrderStatus::Filled
            | BybitOrderStatus::Cancelled
            | BybitOrderStatus::Triggered
            | BybitOrderStatus::Deactivated => AccountEventKind::OrderCancelled::<
                ExchangeId,
                AssetNameExchange,
                InstrumentNameExchange,
            >(OrderResponseCancel {
                key,
                state: Ok(Cancelled {
                    id: order.exchange_order_id,
                    time_exchange,
                }),
            }),
        };

        Ok(AccountEvent { exchange, kind })
    }
}

#[serde_as]
#[derive(Clone, PartialEq, Debug, Deserialize)]
pub struct OrderExecutionData {
    #[serde(rename = "category")]
    pub category: InstrumentCategory,

    #[serde(rename = "symbol")]
    pub symbol: InstrumentNameExchange,

    #[serde(rename = "orderId")]
    pub exchange_order_id: OrderId,

    #[serde_as(as = "DefaultOnError<Option<DisplayFromStr>>")]
    #[serde(rename = "orderLinkId")]
    pub client_order_id: Option<String>,

    #[serde(rename = "execId")]
    pub trade_id: TradeId,

    #[serde(rename = "side")]
    pub side: Side,

    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "feeRate")]
    pub fee_rate: f64,

    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "execPrice")]
    pub exec_price: f64,

    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "execQty")]
    pub exec_qty: f64,

    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "execFee")]
    pub exec_fee: f64,

    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "orderQty")]
    pub order_qty: f64,

    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "leavesQty")]
    pub remaining_qty: f64,

    #[serde(rename = "isMaker")]
    pub is_maker: bool,
}

impl TryFrom<(ExchangeId, OrderExecutionData, DateTime<Utc>)> for UnindexedAccountEvent {
    type Error = SocketError;

    fn try_from(
        (exchange, execution, time_exchange): (ExchangeId, OrderExecutionData, DateTime<Utc>),
    ) -> Result<Self, Self::Error> {
        // TODO: The fee is specified by the instrument type being traded and ExchangeId
        // // https://bybit-exchange.github.io/docs/v5/enum#spot-fee-currency-instruction
        // let fee_asset = match (execution.fee_rate, execution.is_maker, execution.side) {
        //     (fee_rate, _, side) if fee_rate > 0.0 => match side {
        //         Side::Buy => BaseAsset,
        //         Side::Sell => QuoteAsset,
        //     },
        //     (fee_rate, is_maker, side) if fee_rate < 0.0 && is_maker => match side {
        //         Side::Buy => QuoteAsset,
        //         Side::Sell => BaseAsset,
        //     },
        //     (fee_rate, is_maker, side) if fee_rate < 0.0 && !is_maker => match side {
        //         Side::Buy => BaseAsset,
        //         Side::Sell => QuoteAsset,
        //     },
        // };

        Ok(AccountEvent {
            exchange,
            kind: AccountEventKind::Trade(Trade {
                id: execution.trade_id,
                order_id: execution.exchange_order_id,
                instrument: execution.symbol,
                strategy: StrategyId::unknown(),
                time_exchange,
                side: execution.side,
                // TODO: The serde should parse directly to a Decimal type
                price: Decimal::from_f64(execution.exec_price).unwrap(),
                quantity: Decimal::from_f64(execution.exec_qty).unwrap(),
                // TODO: This fee asset type is not correct
                fees: AssetFees {
                    asset: QuoteAsset,
                    fees: Decimal::from_f64(execution.exec_fee).unwrap(),
                },
            }),
        })
    }
}

// TODO: Add tests for deserialization
