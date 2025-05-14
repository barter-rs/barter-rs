use std::borrow::Cow;

use ::serde::{Deserialize, Serialize};
use barter_instrument::instrument::name::InstrumentNameExchange;
use barter_integration::protocol::http::rest::RestRequest;
use derive_more::derive::Constructor;
use reqwest::Method;
use serde_with::{serde_as, skip_serializing_none};

use crate::{
    client::bybit::{http::BybitHttpResponse, types::InstrumentCategory},
    order::id::{ClientOrderId, OrderId},
};

/// https://bybit-exchange.github.io/docs/v5/order/cancel-order
#[derive(Debug, Clone, Constructor)]
pub struct CancelOrderRequest(CancelOrderBody);

impl RestRequest for CancelOrderRequest {
    type Response = CancelOrderResponse;
    type QueryParams = ();
    type Body = CancelOrderBody;

    fn path(&self) -> Cow<'static, str> {
        "/v5/order/cancel".into()
    }

    fn method() -> Method {
        Method::POST
    }

    fn body(&self) -> Option<&Self::Body> {
        Some(&self.0)
    }
}

pub type CancelOrderResponse = BybitHttpResponse<CancelOrderResponseInner>;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct CancelOrderResponseInner {
    #[serde(rename = "orderId")]
    pub exchange_order_id: OrderId,

    #[serde(rename = "orderLinkId")]
    pub client_order_id: ClientOrderId,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize)]
pub struct CancelOrderBody {
    #[serde(rename = "category")]
    pub category: InstrumentCategory,

    #[serde(rename = "symbol")]
    pub instrument: InstrumentNameExchange,

    #[serde(rename = "orderId")]
    pub exchange_order_id: Option<OrderId>,

    #[serde(rename = "orderLinkId")]
    pub client_order_id: Option<ClientOrderId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use std::str::FromStr;

        use chrono::DateTime;

        use super::*;

        #[test]
        fn test_place_order() {
            let raw_response = r#"{
                "retCode": 0,
                "retMsg": "OK",
                "result": {
                    "orderId": "c6f055d9-7f21-4079-913d-e6523a9cfffa",
                    "orderLinkId": "linear-004"
                },
                "retExtInfo": {},
                "time": 1672217377164
            }"#;

            let actual = serde_json::from_str::<CancelOrderResponse>(raw_response).unwrap();

            let expected = CancelOrderResponse {
                ret_code: 0,
                ret_msg: "OK".to_string(),
                time: DateTime::from_str("2022-12-28T08:49:37.164Z").unwrap(),
                result: CancelOrderResponseInner {
                    exchange_order_id: OrderId::new("c6f055d9-7f21-4079-913d-e6523a9cfffa"),
                    client_order_id: ClientOrderId::new("linear-004"),
                },
            };

            assert_eq!(actual, expected);
        }
    }
}
