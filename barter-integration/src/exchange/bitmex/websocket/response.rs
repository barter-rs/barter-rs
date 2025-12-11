use crate::exchange::bitmex::websocket::request::BitmexRequest;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct BitmexResponseSubscribe {
    pub success: bool,
    pub subscribe: SmolStr,
    pub request: BitmexRequest,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct BitmexResponseUnsubscribe {
    pub success: bool,
    pub subscribe: SmolStr,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exchange::bitmex::websocket::request::{BitmexArg, BitmexOperation};
    use smol_str::ToSmolStr;

    #[test]
    fn test_bitmex_response_subscribe() {
        let input = r#"
        {
            "success": true,
            "subscribe": "orderBookL2_25:XBTUSD",
            "request": {
                "op":"subscribe",
                "args":[
                    "orderBookL2_25:XBTUSD"
                ]
            }
        }
        "#;
        assert_eq!(
            serde_json::from_str::<BitmexResponseSubscribe>(&input).unwrap(),
            BitmexResponseSubscribe {
                success: true,
                subscribe: "orderBookL2_25:XBTUSD".to_smolstr(),
                request: BitmexRequest {
                    op: BitmexOperation::Subscribe,
                    args: Some(BitmexArg::VecStr(vec![
                        "orderBookL2_25:XBTUSD".to_smolstr()
                    ])),
                }
            }
        );

        let input = r#"
        {
            "success": false,
            "subscribe": "orderBookL2_25:XBTUSD",
            "request": {
                "op":"subscribe",
                "args":[
                    "orderBookL2_25:XBTUSD"
                ]
            }
        }
        "#;
        assert_eq!(
            serde_json::from_str::<BitmexResponseSubscribe>(&input).unwrap(),
            BitmexResponseSubscribe {
                success: false,
                subscribe: "orderBookL2_25:XBTUSD".to_smolstr(),
                request: BitmexRequest {
                    op: BitmexOperation::Subscribe,
                    args: Some(BitmexArg::VecStr(vec![
                        "orderBookL2_25:XBTUSD".to_smolstr()
                    ])),
                }
            }
        );
    }
}
