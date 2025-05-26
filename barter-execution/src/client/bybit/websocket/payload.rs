use std::{fmt::Display, str::FromStr};

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
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use serde_with::{DefaultOnError, DisplayFromStr, NoneAsEmptyString, serde_as};
use tracing::warn;

use crate::{
    AccountEvent, AccountEventKind, UnindexedAccountEvent, UnindexedAccountEventKind,
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

    #[serde_as(as = "NoneAsEmptyString")]
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
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct OrderExecutionData {
    #[serde(rename = "category")]
    pub category: InstrumentCategory,

    #[serde(rename = "symbol")]
    pub symbol: InstrumentNameExchange,

    #[serde(rename = "orderId")]
    pub exchange_order_id: OrderId,

    #[serde_as(as = "NoneAsEmptyString")]
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
    #[serde(rename = "execType")]
    pub exec_type: ExecType,

    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "orderQty")]
    pub order_qty: f64,

    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "leavesQty")]
    pub remaining_qty: f64,

    #[serde(rename = "isMaker")]
    pub is_maker: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
pub enum ExecType {
    /// Regular trade
    Trade,
    /// https://www.bybit.com/en/help-center/article/Auto-Deleveraging-ADL
    AdlTrade,
    /// https://www.bybit.com/en/help-center/article/Introduction-to-Funding-Rate
    Funding,
    /// Takeover liquidation
    BustTrade,
    /// USDC futures delivery; Position closed by contract delisted
    Delivery,
    /// Inverse futures settlement; Position closed due to delisting
    Settle,
    BlockTrade,
    MovePosition,
    /// Spread leg execution
    FutureSpread,
}

impl Display for ExecType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            serde_json::to_string(self).expect("ExecType is to JSON")
        )
    }
}

impl FromStr for ExecType {
    type Err = Box<dyn std::error::Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Trade" => Ok(ExecType::Trade),
            "AdlTrade" => Ok(ExecType::AdlTrade),
            "Funding" => Ok(ExecType::Funding),
            "BustTrade" => Ok(ExecType::BustTrade),
            "Delivery" => Ok(ExecType::Delivery),
            "Settle" => Ok(ExecType::Settle),
            "BlockTrade" => Ok(ExecType::BlockTrade),
            "MovePosition" => Ok(ExecType::MovePosition),
            "FutureSpread" => Ok(ExecType::FutureSpread),
            _ => Err(format!("execType {s} not supported").into()),
        }
    }
}

impl TryFrom<(ExchangeId, OrderExecutionData, DateTime<Utc>)> for UnindexedAccountEvent {
    type Error = SocketError;

    fn try_from(
        (exchange, execution, time_exchange): (ExchangeId, OrderExecutionData, DateTime<Utc>),
    ) -> Result<Self, Self::Error> {
        let kind = match execution.exec_type {
            ExecType::Trade => handle_normal_trade(execution, time_exchange),
            _ => {
                warn!(?execution, "execution type not handled");

                return Err(SocketError::Unsupported {
                    entity: "AccountEvent".to_string(),
                    item: serde_json::to_string(&execution)
                        .map_err(|err| SocketError::Serialise(err))?,
                });
            }
        };

        Ok(UnindexedAccountEvent { exchange, kind })
    }
}

fn handle_normal_trade(
    execution: OrderExecutionData,
    time_exchange: DateTime<Utc>,
) -> UnindexedAccountEventKind {
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

    AccountEventKind::Trade(Trade {
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
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;
        use barter_integration::{de::datetime_utc_from_epoch_duration, error::SocketError};
        use rust_decimal_macros::dec;
        use std::time::Duration;

        #[test]
        fn test_bybit_payload() {
            struct TestCase {
                input: &'static str,
                expected: Result<BybitPayload, SocketError>,
            }

            let tests = vec![
                // TC0: input BybitPayload is deserialised
                TestCase {
                    input: r#"
                        {
                            "topic": "execution",
                            "id": "386825804_BTCUSDT_140612148849382",
                            "creationTime": 1746270400355,
                            "data": [{"hello":"world"}]
                        }
                    "#,
                    expected: Ok(BybitPayload {
                        topic: BybitPayloadTopic::Execution,
                        timestamp: datetime_utc_from_epoch_duration(Duration::from_millis(
                            1746270400355,
                        )),
                        data: RawValue::from_string("[{\"hello\":\"world\"}]".to_string()).unwrap(),
                    }),
                },
            ];

            for (index, test) in tests.into_iter().enumerate() {
                let actual = serde_json::from_str::<BybitPayload>(test.input);
                match (actual, test.expected) {
                    (Ok(actual), Ok(expected)) => {
                        assert_eq!(actual.topic, expected.topic, "TC{} topic failed", index);
                        assert_eq!(
                            actual.timestamp, expected.timestamp,
                            "TC{} topic failed",
                            index
                        );
                        assert_eq!(
                            actual.data.get(),
                            expected.data.get(),
                            "TC{} topic failed",
                            index
                        );
                    }
                    (Err(_), Err(_)) => {
                        // Test passed
                    }
                    (actual, expected) => {
                        // Test failed
                        panic!(
                            "TC{index} failed because actual != expected. \nActual: {actual:?}\nExpected: {expected:?}\n"
                        );
                    }
                }
            }
        }

        #[test]
        fn test_bybit_execution_data() {
            struct TestCase {
                input: &'static str,
                expected: Result<OrderExecutionData, SocketError>,
            }

            let tests = vec![
                // T0: input Futures execution data
                TestCase {
                    input: r#"
                    {
                        "category": "linear",
                        "symbol": "BTCUSDT",
                        "closedSize": "0.5",
                        "execFee": "26.3725275",
                        "execId": "0ab1bdf7-4219-438b-b30a-32ec863018f7",
                        "execPrice": "95900.1",
                        "execQty": "0.5",
                        "execType": "Trade",
                        "execValue": "47950.05",
                        "feeRate": "0.00055",
                        "tradeIv": "",
                        "markIv": "",
                        "blockTradeId": "",
                        "markPrice": "95901.48",
                        "indexPrice": "",
                        "underlyingPrice": "",
                        "leavesQty": "0",
                        "orderId": "9aac161b-8ed6-450d-9cab-c5cc67c21784",
                        "orderLinkId": "",
                        "orderPrice": "94942.5",
                        "orderQty": "0.5",
                        "orderType": "Market",
                        "stopOrderType": "UNKNOWN",
                        "side": "Sell",
                        "execTime": "1746270400353",
                        "isLeverage": "0",
                        "isMaker": false,
                        "seq": 140612148849382,
                        "marketUnit": "",
                        "execPnl": "0.05",
                        "createType": "CreateByUser"
                    }
                "#,
                    expected: Ok(OrderExecutionData {
                        category: InstrumentCategory::Linear,
                        symbol: InstrumentNameExchange::new("BTCUSDT"),
                        exchange_order_id: OrderId::new("9aac161b-8ed6-450d-9cab-c5cc67c21784"),
                        client_order_id: None,
                        trade_id: TradeId::new("0ab1bdf7-4219-438b-b30a-32ec863018f7"),
                        side: Side::Sell,
                        fee_rate: 0.00055,
                        exec_price: dec!(95900.1),
                        exec_qty: dec!(0.5),
                        exec_fee: dec!(26.3725275),
                        exec_type: ExecType::Trade,
                        order_qty: 0.5,
                        remaining_qty: 0.0,
                        is_maker: false,
                    }),
                },
                // T1: input Spot execution data
                TestCase {
                    input: r#"
                    {
                        "category": "spot",
                        "symbol": "OPUSDT",
                        "closedSize": "",
                        "execFee": "0.0714496",
                        "execId": "2220000000718109300",
                        "execPrice": "0.64",
                        "execQty": "111.64",
                        "execType": "Trade",
                        "execValue": "71.44960",
                        "feeRate": "0.001",
                        "tradeIv": "",
                        "markIv": "",
                        "blockTradeId": "",
                        "markPrice": "",
                        "indexPrice": "",
                        "underlyingPrice": "",
                        "leavesQty": "156.36",
                        "orderId": "1969634974663799040",
                        "orderLinkId": "j_7KyOEUqWio2Ayr6aiF35N",
                        "orderPrice": "0.640",
                        "orderQty": "268.00",
                        "orderType": "Limit",
                        "stopOrderType": "",
                        "side": "Sell",
                        "execTime": "1749534841674",
                        "isLeverage": "0",
                        "isMaker": true,
                        "seq": 105530984287,
                        "marketUnit": ""
                    }
                "#,
                    expected: Ok(OrderExecutionData {
                        category: InstrumentCategory::Spot,
                        symbol: InstrumentNameExchange::new("OPUSDT"),
                        exchange_order_id: OrderId::new("1969634974663799040"),
                        client_order_id: Some("j_7KyOEUqWio2Ayr6aiF35N".to_string()),
                        trade_id: TradeId::new("2220000000718109300"),
                        side: Side::Sell,
                        fee_rate: 0.001,
                        exec_price: dec!(0.64),
                        exec_qty: dec!(111.64),
                        exec_fee: dec!(0.0714496),
                        exec_type: ExecType::Trade,
                        order_qty: 268.00,
                        remaining_qty: 156.36,
                        is_maker: true,
                    }),
                },
            ];

            for (index, test) in tests.into_iter().enumerate() {
                let actual = serde_json::from_str::<OrderExecutionData>(test.input);

                match (actual, test.expected) {
                    (Ok(actual), Ok(expected)) => {
                        assert_eq!(actual, expected, "TC{} topic failed", index);
                    }
                    (Err(_), Err(_)) => {
                        // Test passed
                    }
                    (actual, expected) => {
                        // Test failed
                        panic!(
                            "TC{index} failed because actual != expected. \nActual: {actual:?}\nExpected: {expected:?}\n"
                        );
                    }
                }
            }
        }
    }
}
