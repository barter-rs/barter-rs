use std::borrow::Cow;

use barter_instrument::{Side, instrument::name::InstrumentNameExchange};
use barter_integration::protocol::http::rest::RestRequest;
use derive_more::derive::Constructor;
use reqwest::Method;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};

use crate::{
    client::bybit::{
        http::{BybitHttpResponse, ResultList},
        types::{BybitOrderTimeInForce, InstrumentCategory},
    },
    order::{
        OrderKind,
        id::{ClientOrderId, OrderId},
    },
};

/// https://bybit-exchange.github.io/docs/v5/order/open-order
#[derive(Debug, Clone, Constructor)]
pub struct GetOpenAndClosedOrders(GetOpenAndClosedOrdersParams);

impl RestRequest for GetOpenAndClosedOrders {
    type Response = GetOpenAndClosedOrdersResponse;
    type QueryParams = GetOpenAndClosedOrdersParams;
    type Body = ();

    fn path(&self) -> Cow<'static, str> {
        "/v5/order/realtime".into()
    }

    fn method() -> Method {
        Method::GET
    }

    fn query_params(&self) -> Option<&Self::QueryParams> {
        Some(&self.0)
    }
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize)]
pub struct GetOpenAndClosedOrdersParams {
    #[serde(rename = "category")]
    pub category: InstrumentCategory,
}

pub type GetOpenAndClosedOrdersResponse =
    BybitHttpResponse<ResultList<GetOpenAndClosedOrdersResponseInner>>;

#[serde_as]
#[derive(Debug, Clone, Deserialize)]
pub struct GetOpenAndClosedOrdersResponseInner {
    #[serde(rename = "orderLinkId")]
    pub client_order_id: Option<ClientOrderId>,

    #[serde(rename = "orderId")]
    pub exchange_order_id: OrderId,

    #[serde(rename = "symbol")]
    pub instrument: InstrumentNameExchange,

    #[serde(rename = "side")]
    pub side: Side,

    #[serde(rename = "orderType")]
    pub kind: OrderKind,

    #[serde(rename = "timeInForce")]
    pub time_in_force: BybitOrderTimeInForce,

    #[serde(rename = "price")]
    pub price: Decimal,

    #[serde(rename = "qty")]
    pub quantity: Decimal,

    #[serde(rename = "cumExecQty")]
    pub filled_quantity: Decimal,
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;

        #[test]
        fn test_get_open_and_closed_orders() {
            let raw_response = r#"{
    "retCode": 0,
    "retMsg": "OK",
    "result": {
        "list": [
            {
                "orderId": "fd4300ae-7847-404e-b947-b46980a4d140",
                "orderLinkId": "test-000005",
                "blockTradeId": "",
                "symbol": "ETHUSDT",
                "price": "1600.00",
                "qty": "0.10",
                "side": "Buy",
                "isLeverage": "",
                "positionIdx": 1,
                "orderStatus": "New",
                "cancelType": "UNKNOWN",
                "rejectReason": "EC_NoError",
                "avgPrice": "0",
                "leavesQty": "0.10",
                "leavesValue": "160",
                "cumExecQty": "0.00",
                "cumExecValue": "0",
                "cumExecFee": "0",
                "timeInForce": "GTC",
                "orderType": "Limit",
                "stopOrderType": "UNKNOWN",
                "orderIv": "",
                "triggerPrice": "0.00",
                "takeProfit": "2500.00",
                "stopLoss": "1500.00",
                "tpTriggerBy": "LastPrice",
                "slTriggerBy": "LastPrice",
                "triggerDirection": 0,
                "triggerBy": "UNKNOWN",
                "lastPriceOnCreated": "",
                "reduceOnly": false,
                "closeOnTrigger": false,
                "smpType": "None",
                "smpGroup": 0,
                "smpOrderId": "",
                "tpslMode": "Full",
                "tpLimitPrice": "",
                "slLimitPrice": "",
                "placeType": "",
                "createdTime": "1684738540559",
                "updatedTime": "1684738540561"
            }
        ],
        "nextPageCursor": "page_args%3Dfd4300ae-7847-404e-b947-b46980a4d140%26symbol%3D6%26",
        "category": "linear"
    },
    "retExtInfo": {},
    "time": 1684765770483
}"#;

            serde_json::from_str::<GetOpenAndClosedOrdersResponse>(raw_response).unwrap();
        }
    }
}
