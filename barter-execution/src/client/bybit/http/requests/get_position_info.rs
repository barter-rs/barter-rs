use std::borrow::Cow;

use barter_instrument::{asset::name::AssetNameExchange, instrument::name::InstrumentNameExchange};
use barter_integration::protocol::http::rest::RestRequest;
use reqwest::Method;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as, skip_serializing_none};

use crate::client::bybit::{
    http::{BybitHttpResponse, ResultList},
    types::{BybitPositionSide, InstrumentCategory},
};

/// https://bybit-exchange.github.io/docs/v5/position
#[derive(Debug, Clone)]
pub struct GetPositionInfoRequest(pub GetPositionInfoParams);

impl RestRequest for GetPositionInfoRequest {
    type Response = GetPositionInfoResponse;
    type QueryParams = GetPositionInfoParams;
    type Body = ();

    fn path(&self) -> Cow<'static, str> {
        "/v5/position/list".into()
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
pub struct GetPositionInfoParams {
    #[serde(rename = "category")]
    pub category: InstrumentCategory,

    #[serde(rename = "symbol")]
    pub instrument: Option<InstrumentNameExchange>,

    #[serde(rename = "settleCoin")]
    pub settle_coin: Option<AssetNameExchange>,

    #[serde(rename = "limit")]
    pub limit: Option<u8>,

    #[serde(rename = "cursor")]
    pub cursor: Option<String>,
}

type GetPositionInfoResponse = BybitHttpResponse<ResultList<GetPositionInfoResponseInner>>;

#[serde_as]
#[derive(Debug, Clone, Deserialize)]
pub struct GetPositionInfoResponseInner {
    #[serde(rename = "symbol")]
    pub instrument: InstrumentNameExchange,

    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "leverage")]
    pub leverage: u8,

    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "size")]
    pub quantity: Decimal,

    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "avgPrice")]
    pub average_price: Decimal,

    #[serde(rename = "positionIdx")]
    pub side: BybitPositionSide,
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;

        #[test]
        fn test_get_position_info() {
            let raw_response = r#"{
    "retCode": 0,
    "retMsg": "OK",
    "result": {
        "nextPageCursor": "MAPOUSDT%2C1720526896198%2C1",
        "category": "linear",
        "list": [
            {
                "symbol": "TIAUSDT",
                "leverage": "10",
                "autoAddMargin": 0,
                "avgPrice": "6.49787162",
                "liqPrice": "",
                "riskLimitValue": "200000",
                "takeProfit": "",
                "positionValue": "192.337",
                "isReduceOnly": false,
                "tpslMode": "Full",
                "riskId": 1,
                "trailingStop": "0",
                "unrealisedPnl": "-0.7362",
                "markPrice": "6.473",
                "adlRankIndicator": 2,
                "cumRealisedPnl": "-0.0384674",
                "positionMM": "2.01857682",
                "createdTime": "1720527000280",
                "positionIdx": 1,
                "positionIM": "19.32890682",
                "seq": 116032414131,
                "updatedTime": "1720527889340",
                "side": "Buy",
                "bustPrice": "",
                "positionBalance": "0",
                "leverageSysUpdatedTime": "",
                "curRealisedPnl": "-0.0384674",
                "size": "29.6",
                "positionStatus": "Normal",
                "mmrSysUpdatedTime": "",
                "stopLoss": "",
                "tradeMode": 0,
                "sessionAvgPrice": ""
            },
            {
                "symbol": "MAPOUSDT",
                "leverage": "10",
                "autoAddMargin": 0,
                "avgPrice": "0.00810727",
                "liqPrice": "",
                "riskLimitValue": "25000",
                "takeProfit": "",
                "positionValue": "162.79392037",
                "isReduceOnly": false,
                "tpslMode": "Full",
                "riskId": 1,
                "trailingStop": "0",
                "unrealisedPnl": "-1.21016037",
                "markPrice": "0.008047",
                "adlRankIndicator": 5,
                "cumRealisedPnl": "-14.33069666",
                "positionMM": "3.3364614",
                "createdTime": "1718893800340",
                "positionIdx": 1,
                "positionIM": "16.35997503",
                "seq": 11191422176,
                "updatedTime": "1720526896198",
                "side": "Buy",
                "bustPrice": "",
                "positionBalance": "0",
                "leverageSysUpdatedTime": "",
                "curRealisedPnl": "1.74455861",
                "size": "20080",
                "positionStatus": "Normal",
                "mmrSysUpdatedTime": "",
                "stopLoss": "",
                "tradeMode": 0,
                "sessionAvgPrice": ""
            }
        ]
    },
    "retExtInfo": {},
    "time": 1720529003833
}"#;

            serde_json::from_str::<GetPositionInfoResponse>(raw_response).unwrap();
        }
    }
}
