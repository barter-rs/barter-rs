use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BitmexInfo {
    pub info: SmolStr,
    pub version: SmolStr,
    pub timestamp: DateTime<Utc>,
    pub docs: SmolStr,
    pub heartbeat_enabled: bool,
    pub limit: BitmexWebsocketLimit,
    pub app_name: SmolStr,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BitmexWebsocketLimit {
    remaining: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use smol_str::ToSmolStr;
    use std::str::FromStr;

    #[test]
    fn test_de_info() {
        let raw = r#"{
            "info":"Welcome to the BitMEX Realtime API.",
            "version":"2.0.0",
            "timestamp":"2025-12-15T12:07:08.577Z",
            "docs":"https://www.bitmex.com/app/wsAPI",
            "heartbeatEnabled":false,
            "limit":{"remaining":719},
            "appName":"ws-feedhandler-56b7bc7b45-qnnbl"
        }"#;

        let actual = serde_json::from_str::<BitmexInfo>(raw).unwrap();

        let expected = BitmexInfo {
            info: "Welcome to the BitMEX Realtime API.".to_smolstr(),
            version: "2.0.0".to_smolstr(),
            timestamp: DateTime::from_str("2025-12-15T12:07:08.577Z").unwrap(),
            docs: "https://www.bitmex.com/app/wsAPI".to_smolstr(),
            heartbeat_enabled: false,
            limit: BitmexWebsocketLimit { remaining: 719 },
            app_name: "ws-feedhandler-56b7bc7b45-qnnbl".to_smolstr(),
        };

        assert_eq!(actual, expected)
    }
}
