use std::borrow::Cow;

use barter_instrument::{Side, instrument::name::InstrumentNameExchange};
use barter_integration::protocol::http::rest::RestRequest;
use chrono::{DateTime, Utc};
use derive_more::derive::Constructor;
use reqwest::Method;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, TimestampMilliSeconds, serde_as, skip_serializing_none};

use crate::{
    client::bybit::{
        http::{BybitHttpResponse, ResultList},
        types::InstrumentCategory,
    },
    order::id::{ClientOrderId, OrderId},
    trade::TradeId,
};

/// https://bybit-exchange.github.io/docs/v5/order/execution
#[derive(Debug, Clone, Constructor)]
pub struct GetOrderTradesRequest(GetOrderTradesParams);

impl RestRequest for GetOrderTradesRequest {
    type Response = GetOrderTradesResponse;
    type QueryParams = GetOrderTradesParams;
    type Body = ();

    fn path(&self) -> Cow<'static, str> {
        "/v5/execution/list".into()
    }

    fn method() -> Method {
        Method::GET
    }

    fn query_params(&self) -> Option<&Self::QueryParams> {
        Some(&self.0)
    }
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Clone)]
pub struct GetOrderTradesParams {
    #[serde(rename = "category")]
    pub category: InstrumentCategory,

    #[serde(rename = "orderLinkId")]
    pub client_order_id: Option<ClientOrderId>,

    #[serde(rename = "limit")]
    pub limit: Option<u8>,

    #[serde(rename = "cursor")]
    pub cursor: Option<String>,
}

type GetOrderTradesResponse = BybitHttpResponse<ResultList<GetOrderTradesResponseInner>>;

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct GetOrderTradesResponseInner {
    #[serde(rename = "execId")]
    pub trade_id: TradeId,

    #[serde(rename = "orderId")]
    pub exchange_order_id: OrderId,

    #[serde(rename = "symbol")]
    pub instrument: InstrumentNameExchange,

    #[serde(rename = "side")]
    pub side: Side,

    #[serde(rename = "execTime")]
    #[serde_as(as = "TimestampMilliSeconds<String>")]
    pub executed_at: DateTime<Utc>,

    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "execPrice")]
    pub exec_price: Decimal,

    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "execQty")]
    pub exec_qty: Decimal,
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;

        #[test]
        fn test_get_order_trades() {
            let raw_response = r#"
            {
                "retCode": 0,
                "retMsg": "OK",
                "result": {
                    "nextPageCursor": "132766%3A2%2C132766%3A2",
                    "category": "linear",
                    "list": [
                        {
                            "symbol": "ETHPERP",
                            "orderType": "Market",
                            "underlyingPrice": "",
                            "orderLinkId": "",
                            "side": "Buy",
                            "indexPrice": "",
                            "orderId": "8c065341-7b52-4ca9-ac2c-37e31ac55c94",
                            "stopOrderType": "UNKNOWN",
                            "leavesQty": "0",
                            "execTime": "1672282722429",
                            "feeCurrency": "",
                            "isMaker": false,
                            "execFee": "0.071409",
                            "feeRate": "0.0006",
                            "execId": "e0cbe81d-0f18-5866-9415-cf319b5dab3b",
                            "tradeIv": "",
                            "blockTradeId": "",
                            "markPrice": "1183.54",
                            "execPrice": "1190.15",
                            "markIv": "",
                            "orderQty": "0.1",
                            "orderPrice": "1236.9",
                            "execValue": "119.015",
                            "execType": "Trade",
                            "execQty": "0.1",
                            "closedSize": "",
                            "seq": 4688002127
                        }
                    ]
                },
                "retExtInfo": {},
                "time": 1672283754510
            }"#;

            serde_json::from_str::<GetOrderTradesResponse>(raw_response).unwrap();
        }
    }
}
