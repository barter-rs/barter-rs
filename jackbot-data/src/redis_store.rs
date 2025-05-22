use crate::{
    books::OrderBook,
    subscription::{book::OrderBookEvent, trade::PublicTrade},
};
use jackbot_instrument::exchange::ExchangeId;
use serde::{Serialize, Deserialize};
use fnv::FnvHashMap;
use std::{sync::{Arc, Mutex}};

/// Storage interface for persisting snapshots, deltas and trades.
pub trait RedisStore: Send + Sync {
    fn store_snapshot(&self, exchange: ExchangeId, instrument: &str, snapshot: &OrderBook);
    fn store_delta(&self, exchange: ExchangeId, instrument: &str, delta: &OrderBookEvent);
    fn store_trade(&self, exchange: ExchangeId, instrument: &str, trade: &PublicTrade);
}

/// In-memory RedisStore used for testing.
#[derive(Clone, Default)]
pub struct InMemoryStore {
    snapshots: Arc<Mutex<FnvHashMap<String, String>>>,
    deltas: Arc<Mutex<FnvHashMap<String, Vec<String>>>>,
    trades: Arc<Mutex<FnvHashMap<String, Vec<String>>>>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn snapshot_key(prefix: &str, exchange: ExchangeId, instrument: &str) -> String {
        format!("{}:{}:{}:snapshot", prefix, exchange, instrument)
    }
    fn delta_key(prefix: &str, exchange: ExchangeId, instrument: &str) -> String {
        format!("{}:{}:{}:deltas", prefix, exchange, instrument)
    }
    fn trade_key(prefix: &str, exchange: ExchangeId, instrument: &str) -> String {
        format!("{}:{}:{}:trades", prefix, exchange, instrument)
    }

    /// Helper used in tests.
    pub fn get_snapshot(&self, exchange: ExchangeId, instrument: &str) -> Option<String> {
        let key = Self::snapshot_key("jb", exchange, instrument);
        self.snapshots.lock().unwrap().get(&key).cloned()
    }

    /// Helper used in tests.
    pub fn delta_len(&self, exchange: ExchangeId, instrument: &str) -> usize {
        let key = Self::delta_key("jb", exchange, instrument);
        self.deltas.lock().unwrap().get(&key).map(|v| v.len()).unwrap_or(0)
    }
}

impl RedisStore for InMemoryStore {
    fn store_snapshot(&self, exchange: ExchangeId, instrument: &str, snapshot: &OrderBook) {
        let json = serde_json::to_string(snapshot).expect("serialise snapshot");
        let key = Self::snapshot_key("jb", exchange, instrument);
        self.snapshots.lock().unwrap().insert(key, json);
    }

    fn store_delta(&self, exchange: ExchangeId, instrument: &str, delta: &OrderBookEvent) {
        let json = serde_json::to_string(delta).expect("serialise delta");
        let key = Self::delta_key("jb", exchange, instrument);
        self.deltas
            .lock()
            .unwrap()
            .entry(key)
            .or_default()
            .push(json);
    }

    fn store_trade(&self, exchange: ExchangeId, instrument: &str, trade: &PublicTrade) {
        let json = serde_json::to_string(trade).expect("serialise trade");
        let key = Self::trade_key("jb", exchange, instrument);
        self.trades
            .lock()
            .unwrap()
            .entry(key)
            .or_default()
            .push(json);
    }
}

/// Redis backed store used in production.
#[derive(Clone)]
pub struct RedisClientStore {
    client: redis::Client,
    prefix: String,
}

impl RedisClientStore {
    pub fn new(url: &str, prefix: impl Into<String>) -> redis::RedisResult<Self> {
        Ok(Self { client: redis::Client::open(url)?, prefix: prefix.into() })
    }

    fn key(&self, suffix: &str, exchange: ExchangeId, instrument: &str) -> String {
        format!("{}:{}:{}:{}", self.prefix, exchange, instrument, suffix)
    }
}

impl RedisStore for RedisClientStore {
    fn store_snapshot(&self, exchange: ExchangeId, instrument: &str, snapshot: &OrderBook) {
        let key = self.key("snapshot", exchange, instrument);
        if let Ok(json) = serde_json::to_string(snapshot) {
            if let Ok(mut conn) = self.client.get_connection() {
                let _ : redis::RedisResult<()> = redis::pipe()
                    .atomic()
                    .set(key, json)
                    .query(&mut conn);
            }
        }
    }

    fn store_delta(&self, exchange: ExchangeId, instrument: &str, delta: &OrderBookEvent) {
        let key = self.key("deltas", exchange, instrument);
        if let Ok(json) = serde_json::to_string(delta) {
            if let Ok(mut conn) = self.client.get_connection() {
                let _ : redis::RedisResult<()> = redis::pipe()
                    .atomic()
                    .cmd("RPUSH")
                    .arg(key)
                    .arg(json)
                    .query(&mut conn);
            }
        }
    }

    fn store_trade(&self, exchange: ExchangeId, instrument: &str, trade: &PublicTrade) {
        let key = self.key("trades", exchange, instrument);
        if let Ok(json) = serde_json::to_string(trade) {
            if let Ok(mut conn) = self.client.get_connection() {
                let _ : redis::RedisResult<()> = redis::pipe()
                    .atomic()
                    .cmd("RPUSH")
                    .arg(key)
                    .arg(json)
                    .query(&mut conn);
            }
        }
    }
}
