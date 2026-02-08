use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Serialize, Deserialize)]
pub struct HttpBinanceSpotConfig {
    #[serde(skip_serializing)]
    pub api_key: String,
    #[serde(skip_serializing)]
    pub api_secret: String,
    pub url: String,
}

impl Default for HttpBinanceSpotConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            api_secret: String::new(),
            url: String::from("https://api.binance.com"),
        }
    }
}

impl fmt::Debug for HttpBinanceSpotConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HttpBinanceSpotConfig")
            .field("api_key", &"*** redacted ***")
            .field("api_secret", &"*** redacted ***")
            .field("url", &self.url)
            .finish()
    }
}

#[derive(Clone)]
pub struct HttpBinanceSpotClient {
    pub client: reqwest::Client,
    pub config: HttpBinanceSpotConfig,
}

impl HttpBinanceSpotClient {
    pub fn new(config: HttpBinanceSpotConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
        }
    }
}

impl fmt::Debug for HttpBinanceSpotClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HttpBinanceSpotClient")
            .field("client", &"reqwest::Client { ... }")
            .field("config", &self.config)
            .finish()
    }
}
