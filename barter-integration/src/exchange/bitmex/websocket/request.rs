use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct BitmexRequest {
    pub op: BitmexOperation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<BitmexArg>,
}

impl BitmexRequest {
    pub fn subscribe<'a>(topics: impl IntoIterator<Item = &'a str>) -> Self {
        Self {
            op: BitmexOperation::Subscribe,
            args: Some(BitmexArg::from_iter(topics)),
        }
    }

    pub fn unsubscribe<'a>(topics: impl IntoIterator<Item = &'a str>) -> Self {
        Self {
            op: BitmexOperation::Unsubscribe,
            args: Some(BitmexArg::from_iter(topics)),
        }
    }

    pub fn ping() -> Self {
        Self {
            op: BitmexOperation::Ping,
            args: None,
        }
    }

    pub fn cancel_all_after(timeout_ms: u32) -> Self {
        Self {
            op: BitmexOperation::CancelAllAfter,
            args: Some(BitmexArg::U32(timeout_ms)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BitmexOperation {
    Subscribe,
    Unsubscribe,
    Ping,
    CancelAllAfter,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(untagged)]
pub enum BitmexArg {
    VecStr(Vec<SmolStr>),
    U32(u32),
}

impl<'a> FromIterator<&'a str> for BitmexArg {
    fn from_iter<T: IntoIterator<Item = &'a str>>(iter: T) -> Self {
        Self::VecStr(iter.into_iter().map(SmolStr::new).collect())
    }
}

#[cfg(test)]
mod tests {
    use crate::exchange::bitmex::websocket::request::BitmexRequest;

    #[test]
    fn test_se_request() {
        let request = BitmexRequest::subscribe(["trade:XBTUSD"]);
        assert_eq!(
            serde_json::to_string(&request).unwrap(),
            r#"{"op":"subscribe","args":["trade:XBTUSD"]}"#
        );

        let request = BitmexRequest::unsubscribe(["trade:XBTUSD", "orderBookL2:XRPUSD"]);
        assert_eq!(
            serde_json::to_string(&request).unwrap(),
            r#"{"op":"unsubscribe","args":["trade:XBTUSD","orderBookL2:XRPUSD"]}"#
        );

        let request = BitmexRequest::ping();
        assert_eq!(serde_json::to_string(&request).unwrap(), r#"{"op":"ping"}"#);

        let request = BitmexRequest::cancel_all_after(60000);
        assert_eq!(
            serde_json::to_string(&request).unwrap(),
            r#"{"op":"cancelAllAfter","args":60000}"#
        );
    }
}
