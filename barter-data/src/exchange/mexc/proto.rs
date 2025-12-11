//! MEXC WebSocket protobuf message definitions.
//!
//! These are hand-written based on the MEXC protobuf definitions from:
//! <https://github.com/mexcdevelop/websocket-proto>

/// Wrapper message containing the channel and body.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PushDataV3ApiWrapper {
    /// Channel name (e.g., "spot@public.limit.depth.v3.api.pb@BTCUSDT@20")
    #[prost(string, tag = "1")]
    pub channel: ::prost::alloc::string::String,

    /// L2 aggregated depth data (tag 313)
    #[prost(message, optional, tag = "313")]
    pub public_aggre_depths: ::core::option::Option<PublicAggreDepthsV3Api>,

    /// L1 limit depth data (tag 303)
    #[prost(message, optional, tag = "303")]
    pub public_limit_depths: ::core::option::Option<PublicLimitDepthsV3Api>,

    /// Incremental depth data (tag 302)
    #[prost(message, optional, tag = "302")]
    pub public_increase_depths: ::core::option::Option<PublicIncreaseDepthsV3Api>,

    /// Optional symbol
    #[prost(string, optional, tag = "3")]
    pub symbol: ::core::option::Option<::prost::alloc::string::String>,

    /// Optional symbol ID
    #[prost(string, optional, tag = "4")]
    pub symbol_id: ::core::option::Option<::prost::alloc::string::String>,

    /// Create timestamp (milliseconds)
    #[prost(int64, optional, tag = "5")]
    pub create_time: ::core::option::Option<i64>,

    /// Send timestamp (milliseconds)
    #[prost(int64, optional, tag = "6")]
    pub send_time: ::core::option::Option<i64>,
}

/// L1 partial depth message (snapshot of top N levels).
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PublicLimitDepthsV3Api {
    #[prost(message, repeated, tag = "1")]
    pub asks: ::prost::alloc::vec::Vec<PublicLimitDepthV3ApiItem>,
    #[prost(message, repeated, tag = "2")]
    pub bids: ::prost::alloc::vec::Vec<PublicLimitDepthV3ApiItem>,
    #[prost(string, tag = "3")]
    pub event_type: ::prost::alloc::string::String,
    #[prost(string, tag = "4")]
    pub version: ::prost::alloc::string::String,
}

/// Single price level for L1 depth.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PublicLimitDepthV3ApiItem {
    #[prost(string, tag = "1")]
    pub price: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub quantity: ::prost::alloc::string::String,
}

/// L2 aggregated depth message (incremental updates).
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PublicAggreDepthsV3Api {
    #[prost(message, repeated, tag = "1")]
    pub asks: ::prost::alloc::vec::Vec<PublicAggreDepthV3ApiItem>,
    #[prost(message, repeated, tag = "2")]
    pub bids: ::prost::alloc::vec::Vec<PublicAggreDepthV3ApiItem>,
    #[prost(string, tag = "3")]
    pub event_type: ::prost::alloc::string::String,
    /// Start version for this update batch
    #[prost(string, tag = "4")]
    pub from_version: ::prost::alloc::string::String,
    /// End version for this update batch
    #[prost(string, tag = "5")]
    pub to_version: ::prost::alloc::string::String,
}

/// Single price level for L2 aggregated depth.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PublicAggreDepthV3ApiItem {
    #[prost(string, tag = "1")]
    pub price: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub quantity: ::prost::alloc::string::String,
}

/// Incremental depth message (may be geo-blocked).
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PublicIncreaseDepthsV3Api {
    #[prost(message, repeated, tag = "1")]
    pub asks: ::prost::alloc::vec::Vec<PublicIncreaseDepthV3ApiItem>,
    #[prost(message, repeated, tag = "2")]
    pub bids: ::prost::alloc::vec::Vec<PublicIncreaseDepthV3ApiItem>,
    #[prost(string, tag = "3")]
    pub event_type: ::prost::alloc::string::String,
    #[prost(string, tag = "4")]
    pub version: ::prost::alloc::string::String,
}

/// Single price level for incremental depth.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PublicIncreaseDepthV3ApiItem {
    #[prost(string, tag = "1")]
    pub price: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub quantity: ::prost::alloc::string::String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use prost::Message;

    #[test]
    fn test_limit_depth_encode_decode() {
        // Test encoding and decoding L1 limit depth
        let limit_depth = PublicLimitDepthsV3Api {
            asks: vec![PublicLimitDepthV3ApiItem {
                price: "100.00".to_string(),
                quantity: "1.000".to_string(),
            }],
            bids: vec![PublicLimitDepthV3ApiItem {
                price: "99.00".to_string(),
                quantity: "2.000".to_string(),
            }],
            event_type: "snapshot".to_string(),
            version: "12345".to_string(),
        };

        let mut buf = Vec::new();
        limit_depth.encode(&mut buf).unwrap();

        let decoded = PublicLimitDepthsV3Api::decode(buf.as_slice()).unwrap();
        assert_eq!(decoded.asks.len(), 1);
        assert_eq!(decoded.bids.len(), 1);
        assert_eq!(decoded.asks[0].price, "100.00");
        assert_eq!(decoded.bids[0].price, "99.00");
        assert_eq!(decoded.version, "12345");
    }

    #[test]
    fn test_aggre_depth_item_encode_decode() {
        // Test encoding and decoding a single L2 depth item
        let item = PublicAggreDepthV3ApiItem {
            price: "50000.50".to_string(),
            quantity: "1.25".to_string(),
        };

        let mut buf = Vec::new();
        item.encode(&mut buf).unwrap();

        let decoded = PublicAggreDepthV3ApiItem::decode(buf.as_slice()).unwrap();
        assert_eq!(decoded.price, "50000.50");
        assert_eq!(decoded.quantity, "1.25");
    }

    #[test]
    fn test_aggre_depths_encode_decode() {
        // Test encoding and decoding a complete L2 aggregated depth message
        let depth = PublicAggreDepthsV3Api {
            asks: vec![
                PublicAggreDepthV3ApiItem {
                    price: "50001.00".to_string(),
                    quantity: "0.5".to_string(),
                },
                PublicAggreDepthV3ApiItem {
                    price: "50002.00".to_string(),
                    quantity: "1.0".to_string(),
                },
            ],
            bids: vec![
                PublicAggreDepthV3ApiItem {
                    price: "50000.00".to_string(),
                    quantity: "2.0".to_string(),
                },
                PublicAggreDepthV3ApiItem {
                    price: "49999.00".to_string(),
                    quantity: "3.0".to_string(),
                },
            ],
            event_type: "depth".to_string(),
            from_version: "1000".to_string(),
            to_version: "1005".to_string(),
        };

        let mut buf = Vec::new();
        depth.encode(&mut buf).unwrap();

        let decoded = PublicAggreDepthsV3Api::decode(buf.as_slice()).unwrap();
        assert_eq!(decoded.asks.len(), 2);
        assert_eq!(decoded.bids.len(), 2);
        assert_eq!(decoded.event_type, "depth");
        assert_eq!(decoded.from_version, "1000");
        assert_eq!(decoded.to_version, "1005");
        assert_eq!(decoded.asks[0].price, "50001.00");
        assert_eq!(decoded.bids[0].price, "50000.00");
    }

    #[test]
    fn test_aggre_depths_empty_levels() {
        // Test L2 message with empty bid/ask levels
        let depth = PublicAggreDepthsV3Api {
            asks: vec![],
            bids: vec![],
            event_type: "depth".to_string(),
            from_version: "100".to_string(),
            to_version: "100".to_string(),
        };

        let mut buf = Vec::new();
        depth.encode(&mut buf).unwrap();

        let decoded = PublicAggreDepthsV3Api::decode(buf.as_slice()).unwrap();
        assert!(decoded.asks.is_empty());
        assert!(decoded.bids.is_empty());
    }

    #[test]
    fn test_wrapper_with_aggre_depths() {
        // Test the full wrapper message with L2 aggregated depths
        let aggre_depth = PublicAggreDepthsV3Api {
            asks: vec![PublicAggreDepthV3ApiItem {
                price: "50001.00".to_string(),
                quantity: "0.5".to_string(),
            }],
            bids: vec![PublicAggreDepthV3ApiItem {
                price: "50000.00".to_string(),
                quantity: "1.0".to_string(),
            }],
            event_type: "depth".to_string(),
            from_version: "12345".to_string(),
            to_version: "12350".to_string(),
        };

        let wrapper = PushDataV3ApiWrapper {
            channel: "spot@public.aggre.depth.v3.api.pb@100ms@BTCUSDT".to_string(),
            public_aggre_depths: Some(aggre_depth),
            public_limit_depths: None,
            public_increase_depths: None,
            symbol: Some("BTCUSDT".to_string()),
            symbol_id: Some("123".to_string()),
            create_time: Some(1700000000000),
            send_time: Some(1700000000001),
        };

        let mut buf = Vec::new();
        wrapper.encode(&mut buf).unwrap();

        let decoded = PushDataV3ApiWrapper::decode(buf.as_slice()).unwrap();
        assert_eq!(
            decoded.channel,
            "spot@public.aggre.depth.v3.api.pb@100ms@BTCUSDT"
        );
        assert!(decoded.public_aggre_depths.is_some());
        assert!(decoded.public_limit_depths.is_none());
        assert!(decoded.public_increase_depths.is_none());
        assert_eq!(decoded.symbol, Some("BTCUSDT".to_string()));
        assert_eq!(decoded.create_time, Some(1700000000000));
        assert_eq!(decoded.send_time, Some(1700000000001));

        let aggre = decoded.public_aggre_depths.unwrap();
        assert_eq!(aggre.from_version, "12345");
        assert_eq!(aggre.to_version, "12350");
        assert_eq!(aggre.asks.len(), 1);
        assert_eq!(aggre.bids.len(), 1);
    }

    #[test]
    fn test_wrapper_with_limit_depths() {
        // Test the full wrapper message with L1 limit depths
        let limit_depth = PublicLimitDepthsV3Api {
            asks: vec![PublicLimitDepthV3ApiItem {
                price: "50001.00".to_string(),
                quantity: "0.5".to_string(),
            }],
            bids: vec![PublicLimitDepthV3ApiItem {
                price: "50000.00".to_string(),
                quantity: "1.0".to_string(),
            }],
            event_type: "snapshot".to_string(),
            version: "12345".to_string(),
        };

        let wrapper = PushDataV3ApiWrapper {
            channel: "spot@public.limit.depth.v3.api.pb@BTCUSDT@20".to_string(),
            public_aggre_depths: None,
            public_limit_depths: Some(limit_depth),
            public_increase_depths: None,
            symbol: Some("BTCUSDT".to_string()),
            symbol_id: None,
            create_time: Some(1700000000000),
            send_time: None,
        };

        let mut buf = Vec::new();
        wrapper.encode(&mut buf).unwrap();

        let decoded = PushDataV3ApiWrapper::decode(buf.as_slice()).unwrap();
        assert_eq!(
            decoded.channel,
            "spot@public.limit.depth.v3.api.pb@BTCUSDT@20"
        );
        assert!(decoded.public_limit_depths.is_some());
        assert!(decoded.public_aggre_depths.is_none());

        let limit = decoded.public_limit_depths.unwrap();
        assert_eq!(limit.version, "12345");
        assert_eq!(limit.event_type, "snapshot");
    }

    #[test]
    fn test_large_version_numbers() {
        // Test with large version numbers (common in production)
        let depth = PublicAggreDepthsV3Api {
            asks: vec![],
            bids: vec![],
            event_type: "depth".to_string(),
            from_version: "9999999999999".to_string(),
            to_version: "10000000000000".to_string(),
        };

        let mut buf = Vec::new();
        depth.encode(&mut buf).unwrap();

        let decoded = PublicAggreDepthsV3Api::decode(buf.as_slice()).unwrap();
        assert_eq!(decoded.from_version, "9999999999999");
        assert_eq!(decoded.to_version, "10000000000000");

        // Verify parsing works
        let from: u64 = decoded.from_version.parse().unwrap();
        let to: u64 = decoded.to_version.parse().unwrap();
        assert_eq!(from, 9999999999999);
        assert_eq!(to, 10000000000000);
    }

    #[test]
    fn test_decimal_price_precision() {
        // Test that decimal prices are preserved correctly
        let item = PublicAggreDepthV3ApiItem {
            price: "0.00000001".to_string(),
            quantity: "1000000000.12345678".to_string(),
        };

        let mut buf = Vec::new();
        item.encode(&mut buf).unwrap();

        let decoded = PublicAggreDepthV3ApiItem::decode(buf.as_slice()).unwrap();
        assert_eq!(decoded.price, "0.00000001");
        assert_eq!(decoded.quantity, "1000000000.12345678");
    }
}
