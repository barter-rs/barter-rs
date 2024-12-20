use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct CoinbaseInternationalMessage<Event> {
    pub channel: String,
    pub client_id: String,
    pub timestamp: DateTime<Utc>,
    pub sequence_num: u64,
    pub events: Vec<Event>,
}
