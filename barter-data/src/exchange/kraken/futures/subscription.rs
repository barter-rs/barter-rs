use barter_integration::{Validator, error::SocketError};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum KrakenFuturesSubResponse {
    Subscribed {
        event: String,
        feed: String,
        product_ids: Vec<String>,
    },
    Error {
        event: String,
        message: String,
    },
}

impl Validator for KrakenFuturesSubResponse {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        match &self {
            KrakenFuturesSubResponse::Subscribed { event, .. } if event == "subscribed" => Ok(self),
            KrakenFuturesSubResponse::Error { message, .. } => {
                Err(SocketError::Subscribe(message.clone()))
            }
            _ => Err(SocketError::Subscribe("Unknown subscription response".to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;

        #[test]
        fn test_kraken_futures_sub_response_subscribed() {
            let json = r#"{
                "event": "subscribed",
                "feed": "trade",
                "product_ids": ["PI_XBTUSD"]
            }"#;

            let response: KrakenFuturesSubResponse = serde_json::from_str(json).unwrap();
            match response {
                KrakenFuturesSubResponse::Subscribed { event, feed, product_ids } => {
                    assert_eq!(event, "subscribed");
                    assert_eq!(feed, "trade");
                    assert_eq!(product_ids, vec!["PI_XBTUSD"]);
                }
                _ => panic!("Expected Subscribed variant"),
            }
        }

        #[test]
        fn test_kraken_futures_sub_response_error() {
            let json = r#"{
                "event": "error",
                "message": "Invalid product"
            }"#;

            let response: KrakenFuturesSubResponse = serde_json::from_str(json).unwrap();
            match response {
                KrakenFuturesSubResponse::Error { event, message } => {
                    assert_eq!(event, "error");
                    assert_eq!(message, "Invalid product");
                }
                _ => panic!("Expected Error variant"),
            }
        }

        #[test]
        fn test_kraken_futures_sub_response_multiple_products() {
            let json = r#"{
                "event": "subscribed",
                "feed": "book",
                "product_ids": ["PI_XBTUSD", "PI_ETHUSD", "PI_SOLUSD"]
            }"#;

            let response: KrakenFuturesSubResponse = serde_json::from_str(json).unwrap();
            match response {
                KrakenFuturesSubResponse::Subscribed { product_ids, .. } => {
                    assert_eq!(product_ids.len(), 3);
                    assert!(product_ids.contains(&"PI_XBTUSD".to_string()));
                    assert!(product_ids.contains(&"PI_ETHUSD".to_string()));
                    assert!(product_ids.contains(&"PI_SOLUSD".to_string()));
                }
                _ => panic!("Expected Subscribed variant"),
            }
        }
    }

    mod validate {
        use super::*;

        #[test]
        fn test_kraken_futures_sub_response_validate_success() {
            let response = KrakenFuturesSubResponse::Subscribed {
                event: "subscribed".to_string(),
                feed: "trade".to_string(),
                product_ids: vec!["PI_XBTUSD".to_string()],
            };

            let result = response.validate();
            assert!(result.is_ok());
        }

        #[test]
        fn test_kraken_futures_sub_response_validate_error() {
            let response = KrakenFuturesSubResponse::Error {
                event: "error".to_string(),
                message: "Invalid product".to_string(),
            };

            let result = response.validate();
            assert!(result.is_err());
            
            if let Err(SocketError::Subscribe(msg)) = result {
                assert_eq!(msg, "Invalid product");
            } else {
                panic!("Expected SocketError::Subscribe");
            }
        }

        #[test]
        fn test_kraken_futures_sub_response_validate_wrong_event() {
            let response = KrakenFuturesSubResponse::Subscribed {
                event: "unsubscribed".to_string(), // Wrong event type
                feed: "trade".to_string(),
                product_ids: vec!["PI_XBTUSD".to_string()],
            };

            let result = response.validate();
            assert!(result.is_err());
        }
    }
}