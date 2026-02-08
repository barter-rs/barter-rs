use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpBinanceSpotConfig {
    pub api_key: String,
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

#[derive(Debug, Clone)]
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
