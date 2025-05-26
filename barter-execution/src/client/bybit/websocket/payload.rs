use barter_instrument::{
    Side,
    asset::{QuoteAsset, name::AssetNameExchange},
    exchange::ExchangeId,
    instrument::name::InstrumentNameExchange,
};
use barter_integration::{
    de::de_u64_epoch_ms_as_datetime_utc, error::SocketError, snapshot::Snapshot,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::value::RawValue;
use serde_with::{DefaultOnError, DisplayFromStr, serde_as};

use crate::{
    AccountEvent, AccountEventKind, UnindexedAccountEvent,
    client::bybit::types::{
        BybitOrderStatus, BybitOrderTimeInForce, BybitOrderType, BybitPositionSide,
        InstrumentCategory,
    },
    error::{ApiError, OrderError},
    order::{
        Order, OrderKey, OrderKind, TimeInForce,
        id::{ClientOrderId, OrderId, StrategyId},
        state::{ActiveOrderState, Cancelled, InactiveOrderState, Open, OrderState},
    },
    trade::{AssetFees, Trade, TradeId},
};

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
    pub original_quantity: Decimal,

    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "price")]
    pub original_price: Decimal,

    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "cumExecQty")]
    pub cumulative_executed_quantity: Decimal,

    #[serde_as(as = "DefaultOnError<Option<DisplayFromStr>>")]
    #[serde(rename = "avgPrice")]
    pub average_price: Option<f64>,
}

impl TryFrom<(ExchangeId, OrderUpdateData, DateTime<Utc>)> for UnindexedAccountEvent {
    type Error = SocketError;

    fn try_from(
        (exchange, order, time_exchange): (ExchangeId, OrderUpdateData, DateTime<Utc>),
    ) -> Result<Self, Self::Error> {
        let cid = order
            .client_order_id
            .map(|cid| ClientOrderId::new(cid))
            .ok_or_else(|| {
                SocketError::Exchange(
                    "Orders without client_order_id are not supported".to_string(),
                )
            })?;

        let key = OrderKey {
            exchange,
            instrument: order.symbol,
            strategy: StrategyId::unknown(),
            cid,
        };

        let state = match order.status {
            BybitOrderStatus::New
            | BybitOrderStatus::PartiallyFilled
            | BybitOrderStatus::Untriggered => {
                OrderState::<AssetNameExchange, _>::active(ActiveOrderState::Open(Open {
                    id: order.exchange_order_id,
                    time_exchange,
                    filled_quantity: order.cumulative_executed_quantity,
                }))
            }
            BybitOrderStatus::Rejected => {
                OrderState::<AssetNameExchange, _>::inactive(InactiveOrderState::OpenFailed(
                    OrderError::Rejected(ApiError::Custom(order.rejection_reason)),
                ))
            }
            BybitOrderStatus::Filled => {
                OrderState::<AssetNameExchange, _>::inactive(InactiveOrderState::FullyFilled)
            }
            BybitOrderStatus::PartiallyFilledCanceled
            | BybitOrderStatus::Cancelled
            | BybitOrderStatus::Triggered
            | BybitOrderStatus::Deactivated => {
                OrderState::Inactive(InactiveOrderState::Cancelled(Cancelled {
                    id: order.exchange_order_id,
                    time_exchange,
                }))
            }
        };

        let snapshot = AccountEventKind::OrderSnapshot(Snapshot(Order {
            key,
            side: order.side,
            price: order.original_price,
            quantity: order.original_quantity,
            kind: OrderKind::from(order.order_type),
            time_in_force: TimeInForce::from(order.time_in_force),
            state,
        }));

        Ok(AccountEvent {
            exchange,
            kind: snapshot,
        })
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
    pub exec_price: Decimal,

    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "execQty")]
    pub exec_qty: Decimal,

    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "execFee")]
    pub exec_fee: Decimal,

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
                price: execution.exec_price,
                quantity: execution.exec_qty,
                // TODO: This fee asset type is not correct. In case of Spot,
                // the fee payed depends on the direction of the trade made. The
                // fee is subtracted from the asset we receive.
                fees: AssetFees {
                    asset: QuoteAsset,
                    fees: execution.exec_fee,
                },
            }),
        })
    }
}

// TODO: Add tests for deserialization
