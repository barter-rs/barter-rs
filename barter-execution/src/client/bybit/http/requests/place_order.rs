use std::borrow::Cow;

use ::serde::{Deserialize, Serialize};
use barter_instrument::{Side, instrument::name::InstrumentNameExchange};
use barter_integration::protocol::http::rest::RestRequest;
use derive_more::derive::Constructor;
use reqwest::Method;
use rust_decimal::Decimal;
use serde_with::{DisplayFromStr, serde_as, skip_serializing_none};

use crate::{
    client::bybit::{
        http::BybitHttpResponse,
        types::{BybitOrderTimeInForce, BybitPositionSide, InstrumentCategory},
    },
    order::{
        OrderKind,
        id::{ClientOrderId, OrderId},
    },
};

/// https://bybit-exchange.github.io/docs/v5/order/create-order
#[derive(Debug, Clone, Constructor)]
pub struct PlaceOrderRequest(PlaceOrderBody);

impl RestRequest for PlaceOrderRequest {
    type Response = PlaceOrderResponse;
    type QueryParams = ();
    type Body = PlaceOrderBody;

    fn path(&self) -> Cow<'static, str> {
        "/v5/order/create".into()
    }

    fn method() -> Method {
        Method::POST
    }

    fn body(&self) -> Option<&Self::Body> {
        Some(&self.0)
    }
}

pub type PlaceOrderResponse = BybitHttpResponse<PlaceOrderResponseInner>;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct PlaceOrderResponseInner {
    #[serde(rename = "orderId")]
    pub exchange_order_id: OrderId,

    #[serde(rename = "orderLinkId")]
    pub client_order_id: ClientOrderId,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize)]
pub struct PlaceOrderBody {
    #[serde(rename = "category")]
    pub category: InstrumentCategory,

    #[serde(rename = "symbol")]
    pub instrument: InstrumentNameExchange,

    #[serde(rename = "side")]
    pub side: Side,

    #[serde(rename = "orderType")]
    pub kind: OrderKind,

    #[serde(rename = "timeInForce")]
    pub time_in_force: BybitOrderTimeInForce,

    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "qty")]
    pub quantity: Decimal,

    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(rename = "price")]
    pub price: Option<Decimal>,

    #[serde(rename = "positionIdx")]
    pub position_side: Option<BybitPositionSide>,

    #[serde(rename = "orderLinkId")]
    pub client_order_id: Option<ClientOrderId>,

    #[serde(rename = "reduceOnly")]
    pub reduce_only: Option<bool>,
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
                    "orderId": "1321003749386327552",
                    "orderLinkId": "test-client-id"
                },
                "retExtInfo": {},
                "time": 1672211918471
            }"#;

            let actual = serde_json::from_str::<PlaceOrderResponse>(raw_response).unwrap();

            let expected = PlaceOrderResponse {
                ret_code: 0,
                ret_msg: "OK".to_string(),
                time: DateTime::from_str("2022-12-28T07:18:38.471Z").unwrap(),
                result: PlaceOrderResponseInner {
                    exchange_order_id: OrderId::new("1321003749386327552"),
                    client_order_id: ClientOrderId::new("test-client-id"),
                },
            };

            assert_eq!(actual, expected);
        }
    }
}
