//! MEXC subscription response handling.

use barter_integration::Validator;
use serde::Deserialize;

/// MEXC subscription response.
///
/// Success: `{"id":0,"code":0,"msg":"spot@public.aggre.depth.v3.api.pb@100ms@BTCUSDT"}`
/// Error: `{"id":0,"code":0,"msg":"Not Subscribed successfully! [...]. Reason: Blocked!"}`
#[derive(Clone, Eq, PartialEq, Debug, Deserialize)]
pub struct MexcSubResponse {
    pub id: i64,
    pub code: i64,
    pub msg: String,
}

impl Validator for MexcSubResponse {
    fn validate(self) -> Result<Self, barter_integration::error::SocketError> {
        // A successful subscription has code=0 and msg contains the channel name
        // An error has msg containing "Not Subscribed successfully"
        if self.code == 0 && !self.msg.contains("Not Subscribed successfully") {
            Ok(self)
        } else {
            Err(barter_integration::error::SocketError::Subscribe(format!(
                "MEXC subscription failed: {}",
                self.msg
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_success() {
        let json = r#"{"id":0,"code":0,"msg":"spot@public.aggre.depth.v3.api.pb@100ms@BTCUSDT"}"#;
        let response: MexcSubResponse = serde_json::from_str(json).unwrap();
        assert!(response.validate().is_ok());
    }

    #[test]
    fn test_deserialize_error() {
        let json = r#"{"id":0,"code":0,"msg":"Not Subscribed successfully! Reason: Blocked!"}"#;
        let response: MexcSubResponse = serde_json::from_str(json).unwrap();
        assert!(response.validate().is_err());
    }
}


