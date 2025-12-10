//! MEXC L2 OrderBook transformer and snapshot fetcher.

use crate::{
    Identifier,
    books::{Level, OrderBook},
    error::DataError,
    event::MarketEvent,
    exchange::{Connector, mexc::{MexcSpot, market::extract_symbol_from_channel, proto::PushDataV3ApiWrapper}},
    instrument::InstrumentData,
    subscription::{Map, Subscription, book::{OrderBookEvent, OrderBooksL2}},
    transformer::ExchangeTransformer,
    SnapshotFetcher,
};
use async_trait::async_trait;
use barter_instrument::exchange::ExchangeId;
use barter_integration::{Transformer, error::SocketError, protocol::websocket::WsMessage, subscription::SubscriptionId};
use chrono::{DateTime, Utc};
use futures_util::future::try_join_all;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::{collections::HashMap, future::Future};
use tokio::sync::mpsc;
use tracing::debug;

/// REST API base URL for MEXC.
pub const MEXC_REST_BASE_URL: &str = "https://api.mexc.com";

/// MEXC REST orderbook snapshot response.
#[derive(Debug, Clone, Deserialize)]
pub struct MexcRestSnapshot {
    #[serde(rename = "lastUpdateId")]
    pub last_update_id: u64,
    pub bids: Vec<[String; 2]>,
    pub asks: Vec<[String; 2]>,
    pub timestamp: Option<u64>,
}

/// Fetch orderbook snapshot from MEXC REST API.
pub async fn fetch_snapshot(symbol: &str, limit: u32) -> Result<MexcRestSnapshot, SocketError> {
    let url = format!(
        "{}/api/v3/depth?symbol={}&limit={}",
        MEXC_REST_BASE_URL, symbol, limit
    );

    let response = reqwest::get(&url).await?;

    if !response.status().is_success() {
        return Err(SocketError::HttpResponse(
            response.status(),
            format!("Failed to fetch snapshot for {}", symbol),
        ));
    }

    response
        .json()
        .await
        .map_err(|e| SocketError::Http(e))
}

/// MEXC L2 snapshot fetcher.
#[derive(Debug)]
pub struct MexcOrderBooksL2SnapshotFetcher;

impl SnapshotFetcher<MexcSpot, OrderBooksL2> for MexcOrderBooksL2SnapshotFetcher {
    fn fetch_snapshots<Instrument>(
        subscriptions: &[Subscription<MexcSpot, Instrument, OrderBooksL2>],
    ) -> impl Future<Output = Result<Vec<MarketEvent<Instrument::Key, OrderBookEvent>>, SocketError>> + Send
    where
        Instrument: InstrumentData,
        Instrument::Key: Clone,
        Subscription<MexcSpot, Instrument, OrderBooksL2>: Identifier<<MexcSpot as Connector>::Market>,
    {
        // Create futures for fetching all snapshots in parallel
        let snapshot_futures = subscriptions.iter().map(|sub| {
            let market = sub.id();
            let symbol = market.0.clone();
            let instrument_key = sub.instrument.key().clone();
            let snapshot_depth = sub.exchange.snapshot_depth;

            async move {
                let snapshot = fetch_snapshot(&symbol, snapshot_depth).await?;

                let ts = snapshot
                    .timestamp
                    .and_then(|ms| DateTime::from_timestamp_millis(ms as i64))
                    .unwrap_or_else(Utc::now);

                let bids: Vec<Level> = snapshot
                    .bids
                    .iter()
                    .filter_map(|level| {
                        let price: Decimal = level[0].parse().ok()?;
                        let amount: Decimal = level[1].parse().ok()?;
                        Some(Level::new(price, amount))
                    })
                    .collect();

                let asks: Vec<Level> = snapshot
                    .asks
                    .iter()
                    .filter_map(|level| {
                        let price: Decimal = level[0].parse().ok()?;
                        let amount: Decimal = level[1].parse().ok()?;
                        Some(Level::new(price, amount))
                    })
                    .collect();

                Ok(MarketEvent {
                    time_exchange: ts,
                    time_received: Utc::now(),
                    exchange: ExchangeId::Mexc,
                    instrument: instrument_key,
                    kind: OrderBookEvent::Snapshot(OrderBook::new(
                        snapshot.last_update_id,
                        Some(ts),
                        bids,
                        asks,
                    )),
                })
            }
        });

        // Execute all snapshot fetches in parallel
        try_join_all(snapshot_futures)
    }
}

/// MEXC L2 sequencer state for a single symbol.
#[derive(Debug)]
struct L2SequencerState<InstrumentKey> {
    instrument_key: InstrumentKey,
    snapshot_version: u64,
    last_version: Option<u64>,
    synced: bool,
}

/// MEXC L2 orderbook transformer with version validation.
#[derive(Debug)]
pub struct MexcOrderBooksL2Transformer<InstrumentKey> {
    instrument_map: HashMap<SubscriptionId, L2SequencerState<InstrumentKey>>,
}

#[async_trait]
impl<InstrumentKey> ExchangeTransformer<MexcSpot, InstrumentKey, OrderBooksL2>
    for MexcOrderBooksL2Transformer<InstrumentKey>
where
    InstrumentKey: Clone + PartialEq + Send + Sync,
{
    async fn init(
        instrument_map: Map<InstrumentKey>,
        initial_snapshots: &[MarketEvent<InstrumentKey, OrderBookEvent>],
        _ws_sink_tx: mpsc::UnboundedSender<WsMessage>,
    ) -> Result<Self, DataError> {
        // Build map from symbol to sequencer state
        let mut map = HashMap::new();

        for (sub_id, instrument_key) in instrument_map.0 {
            // Find snapshot matching this specific instrument key
            let snapshot = initial_snapshots
                .iter()
                .find(|event| event.instrument == instrument_key)
                .ok_or_else(|| DataError::InitialSnapshotMissing(sub_id.clone()))?;

            let OrderBookEvent::Snapshot(book) = &snapshot.kind else {
                return Err(DataError::InitialSnapshotInvalid(String::from(
                    "expected OrderBookEvent::Snapshot but found OrderBookEvent::Update",
                )));
            };

            map.insert(
                sub_id,
                L2SequencerState {
                    instrument_key,
                    snapshot_version: book.sequence(),
                    last_version: None,
                    synced: false,
                },
            );
        }

        Ok(Self { instrument_map: map })
    }
}

impl<InstrumentKey> Transformer for MexcOrderBooksL2Transformer<InstrumentKey>
where
    InstrumentKey: Clone,
{
    type Error = DataError;
    type Input = PushDataV3ApiWrapper;
    type Output = MarketEvent<InstrumentKey, OrderBookEvent>;
    type OutputIter = Vec<Result<Self::Output, Self::Error>>;

    fn transform(&mut self, input: Self::Input) -> Self::OutputIter {
        // Extract symbol from channel
        let symbol = match extract_symbol_from_channel(&input.channel) {
            Some(s) => s,
            None => return vec![],
        };

        // Find sequencer state using SubscriptionId
        // SubscriptionId format: "{channel}|{market}" e.g., "aggre.depth|ETHUSDT"
        let sub_id = SubscriptionId::from(format!("aggre.depth|{}", symbol));
        let state = match self.instrument_map.get_mut(&sub_id) {
            Some(s) => s,
            None => return vec![],
        };

        // Get aggregated depth data
        let aggre_depth = match input.public_aggre_depths {
            Some(data) => data,
            None => return vec![],
        };

        // Parse versions
        let from_version: u64 = match aggre_depth.from_version.parse() {
            Ok(v) => v,
            Err(_) => return vec![],
        };
        let to_version: u64 = match aggre_depth.to_version.parse() {
            Ok(v) => v,
            Err(_) => return vec![],
        };

        // Parse timestamp
        let ts = input
            .send_time
            .or(input.create_time)
            .and_then(|ms| DateTime::from_timestamp_millis(ms))
            .unwrap_or_else(Utc::now);

        // Validate version sequence
        let was_synced = state.synced;
        match state.validate_update(from_version, to_version) {
            L2ValidationResult::Outdated => {
                debug!(
                    %symbol,
                    from_version,
                    to_version,
                    snapshot_version = state.snapshot_version,
                    "Skipping outdated update"
                );
                return vec![];
            }
            L2ValidationResult::Gap { prev_last_update_id, first_update_id } => {
                return vec![Err(DataError::InvalidSequence {
                    prev_last_update_id,
                    first_update_id,
                })];
            }
            L2ValidationResult::Valid => {
                if !was_synced && state.synced {
                    debug!(
                        %symbol,
                        from_version,
                        to_version,
                        snapshot_version = state.snapshot_version,
                        "L2 stream synced"
                    );
                }
            }
        }

        // Build orderbook update
        let bids: Vec<Level> = aggre_depth
            .bids
            .iter()
            .filter_map(|level| {
                let price: Decimal = level.price.parse().ok()?;
                let amount: Decimal = level.quantity.parse().ok()?;
                Some(Level::new(price, amount))
            })
            .collect();

        let asks: Vec<Level> = aggre_depth
            .asks
            .iter()
            .filter_map(|level| {
                let price: Decimal = level.price.parse().ok()?;
                let amount: Decimal = level.quantity.parse().ok()?;
                Some(Level::new(price, amount))
            })
            .collect();

        vec![Ok(MarketEvent {
            time_exchange: ts,
            time_received: Utc::now(),
            exchange: ExchangeId::Mexc,
            instrument: state.instrument_key.clone(),
            kind: OrderBookEvent::Update(OrderBook::new(to_version, Some(ts), bids, asks)),
        })]
    }
}

/// Validation result for L2 sequencer.
#[derive(Debug, Clone, PartialEq)]
pub enum L2ValidationResult {
    /// Update is valid and should be applied
    Valid,
    /// Update is outdated and should be skipped
    Outdated,
    /// Sequence gap detected, requires resync
    Gap { prev_last_update_id: u64, first_update_id: u64 },
}

impl<InstrumentKey> L2SequencerState<InstrumentKey> {
    /// Create a new sequencer state for testing.
    #[cfg(test)]
    pub fn new_for_test(instrument_key: InstrumentKey, snapshot_version: u64) -> Self {
        Self {
            instrument_key,
            snapshot_version,
            last_version: None,
            synced: false,
        }
    }

    /// Validate an update and return the result.
    /// This encapsulates the version validation logic per MEXC docs:
    /// - toVersion < snapshot.lastUpdateId → Discard (outdated)
    /// - fromVersion > snapshot.lastUpdateId → Resync (gap detected)
    /// - fromVersion <= snapshot.lastUpdateId <= toVersion → Apply update (first sync)
    /// - Subsequent: fromVersion > lastToVersion + 1 → Resync (sequence break)
    pub fn validate_update(&mut self, from_version: u64, to_version: u64) -> L2ValidationResult {
        if !self.synced {
            // First update after snapshot
            if to_version < self.snapshot_version {
                return L2ValidationResult::Outdated;
            }

            // Check for valid overlap: fromVersion <= snapshot.lastUpdateId <= toVersion
            if from_version > self.snapshot_version {
                // Gap detected - snapshot is too old, need to resync
                return L2ValidationResult::Gap {
                    prev_last_update_id: self.snapshot_version,
                    first_update_id: from_version,
                };
            }

            self.synced = true;
        } else {
            // Subsequent updates: check for continuous sequence
            if let Some(last) = self.last_version {
                if to_version <= last {
                    // Outdated, skip silently
                    return L2ValidationResult::Outdated;
                }
                // MEXC uses version ranges, so fromVersion should be <= last + 1
                // (allows for some overlap in ranges)
                if from_version > last + 1 {
                    return L2ValidationResult::Gap {
                        prev_last_update_id: last,
                        first_update_id: from_version,
                    };
                }
            }
        }

        self.last_version = Some(to_version);
        L2ValidationResult::Valid
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod l2_sequencer_state {
        use super::*;

        #[test]
        fn test_first_update_outdated() {
            // TC: toVersion < snapshot.lastUpdateId → Discard
            let mut state = L2SequencerState::new_for_test("BTCUSDT", 100);

            let result = state.validate_update(50, 90); // to_version(90) < snapshot(100)
            assert_eq!(result, L2ValidationResult::Outdated);
            assert!(!state.synced);
            assert_eq!(state.last_version, None);
        }

        #[test]
        fn test_first_update_gap_detected() {
            // TC: fromVersion > snapshot.lastUpdateId → Gap
            let mut state = L2SequencerState::new_for_test("BTCUSDT", 100);

            let result = state.validate_update(110, 120); // from_version(110) > snapshot(100)
            assert_eq!(
                result,
                L2ValidationResult::Gap {
                    prev_last_update_id: 100,
                    first_update_id: 110,
                }
            );
            assert!(!state.synced);
        }

        #[test]
        fn test_first_update_valid_exact_overlap() {
            // TC: fromVersion <= snapshot.lastUpdateId <= toVersion (exact match)
            let mut state = L2SequencerState::new_for_test("BTCUSDT", 100);

            let result = state.validate_update(100, 110); // from(100) <= snap(100) <= to(110)
            assert_eq!(result, L2ValidationResult::Valid);
            assert!(state.synced);
            assert_eq!(state.last_version, Some(110));
        }

        #[test]
        fn test_first_update_valid_with_buffer() {
            // TC: fromVersion < snapshot.lastUpdateId < toVersion
            let mut state = L2SequencerState::new_for_test("BTCUSDT", 100);

            let result = state.validate_update(90, 110); // from(90) < snap(100) < to(110)
            assert_eq!(result, L2ValidationResult::Valid);
            assert!(state.synced);
            assert_eq!(state.last_version, Some(110));
        }

        #[test]
        fn test_first_update_valid_from_equals_snapshot() {
            // TC: fromVersion == snapshot.lastUpdateId
            let mut state = L2SequencerState::new_for_test("BTCUSDT", 100);

            let result = state.validate_update(100, 105);
            assert_eq!(result, L2ValidationResult::Valid);
            assert!(state.synced);
            assert_eq!(state.last_version, Some(105));
        }

        #[test]
        fn test_subsequent_update_valid_continuous() {
            // TC: Valid continuous sequence after sync
            let mut state = L2SequencerState::new_for_test("BTCUSDT", 100);
            
            // First sync
            let _ = state.validate_update(95, 110);
            assert!(state.synced);

            // Subsequent update: fromVersion == last + 1
            let result = state.validate_update(111, 120);
            assert_eq!(result, L2ValidationResult::Valid);
            assert_eq!(state.last_version, Some(120));
        }

        #[test]
        fn test_subsequent_update_valid_with_overlap() {
            // TC: Valid update with overlapping version range
            let mut state = L2SequencerState::new_for_test("BTCUSDT", 100);
            
            // First sync
            let _ = state.validate_update(95, 110);

            // Subsequent update: fromVersion <= last + 1 (overlapping range)
            let result = state.validate_update(110, 125);
            assert_eq!(result, L2ValidationResult::Valid);
            assert_eq!(state.last_version, Some(125));
        }

        #[test]
        fn test_subsequent_update_outdated() {
            // TC: toVersion <= last → Outdated
            let mut state = L2SequencerState::new_for_test("BTCUSDT", 100);
            
            // First sync
            let _ = state.validate_update(95, 110);

            // Outdated update: toVersion <= last
            let result = state.validate_update(100, 110); // to(110) == last(110)
            assert_eq!(result, L2ValidationResult::Outdated);
            assert_eq!(state.last_version, Some(110)); // unchanged
        }

        #[test]
        fn test_subsequent_update_outdated_older() {
            // TC: toVersion < last → Outdated
            let mut state = L2SequencerState::new_for_test("BTCUSDT", 100);
            
            // First sync
            let _ = state.validate_update(95, 110);

            // Very old update
            let result = state.validate_update(80, 90);
            assert_eq!(result, L2ValidationResult::Outdated);
            assert_eq!(state.last_version, Some(110)); // unchanged
        }

        #[test]
        fn test_subsequent_update_gap_detected() {
            // TC: fromVersion > last + 1 → Gap
            let mut state = L2SequencerState::new_for_test("BTCUSDT", 100);
            
            // First sync
            let _ = state.validate_update(95, 110);

            // Gap: fromVersion > last + 1
            let result = state.validate_update(115, 125); // from(115) > last(110) + 1
            assert_eq!(
                result,
                L2ValidationResult::Gap {
                    prev_last_update_id: 110,
                    first_update_id: 115,
                }
            );
        }

        #[test]
        fn test_subsequent_update_boundary_valid() {
            // TC: fromVersion == last + 1 (exact boundary)
            let mut state = L2SequencerState::new_for_test("BTCUSDT", 100);
            
            // First sync
            let _ = state.validate_update(95, 110);

            // Exact boundary: fromVersion == last + 1
            let result = state.validate_update(111, 120);
            assert_eq!(result, L2ValidationResult::Valid);
            assert_eq!(state.last_version, Some(120));
        }

        #[test]
        fn test_multiple_sequential_updates() {
            // TC: Multiple valid sequential updates
            let mut state = L2SequencerState::new_for_test("BTCUSDT", 100);
            
            // First sync
            assert_eq!(state.validate_update(95, 110), L2ValidationResult::Valid);
            assert_eq!(state.last_version, Some(110));

            // Second update
            assert_eq!(state.validate_update(111, 120), L2ValidationResult::Valid);
            assert_eq!(state.last_version, Some(120));

            // Third update
            assert_eq!(state.validate_update(121, 130), L2ValidationResult::Valid);
            assert_eq!(state.last_version, Some(130));

            // Fourth update with overlap
            assert_eq!(state.validate_update(130, 140), L2ValidationResult::Valid);
            assert_eq!(state.last_version, Some(140));
        }

        #[test]
        fn test_gap_after_multiple_updates() {
            // TC: Gap detected after several successful updates
            let mut state = L2SequencerState::new_for_test("BTCUSDT", 100);
            
            let _ = state.validate_update(95, 110);
            let _ = state.validate_update(111, 120);
            let _ = state.validate_update(121, 130);

            // Now a gap
            let result = state.validate_update(150, 160);
            assert_eq!(
                result,
                L2ValidationResult::Gap {
                    prev_last_update_id: 130,
                    first_update_id: 150,
                }
            );
        }
    }

    mod de {
        use super::*;

        #[test]
        fn test_mexc_rest_snapshot_deserialize() {
            let input = r#"
            {
                "lastUpdateId": 1234567890,
                "bids": [
                    ["50000.00", "1.5"],
                    ["49999.00", "2.0"]
                ],
                "asks": [
                    ["50001.00", "0.5"],
                    ["50002.00", "1.0"]
                ],
                "timestamp": 1700000000000
            }
            "#;

            let snapshot: MexcRestSnapshot = serde_json::from_str(input).unwrap();
            assert_eq!(snapshot.last_update_id, 1234567890);
            assert_eq!(snapshot.bids.len(), 2);
            assert_eq!(snapshot.asks.len(), 2);
            assert_eq!(snapshot.bids[0][0], "50000.00");
            assert_eq!(snapshot.bids[0][1], "1.5");
            assert_eq!(snapshot.timestamp, Some(1700000000000));
        }

        #[test]
        fn test_mexc_rest_snapshot_without_timestamp() {
            let input = r#"
            {
                "lastUpdateId": 1234567890,
                "bids": [],
                "asks": []
            }
            "#;

            let snapshot: MexcRestSnapshot = serde_json::from_str(input).unwrap();
            assert_eq!(snapshot.last_update_id, 1234567890);
            assert!(snapshot.bids.is_empty());
            assert!(snapshot.asks.is_empty());
            assert_eq!(snapshot.timestamp, None);
        }
    }

    mod integration {
        use super::*;
        use crate::subscription::book::OrderBookEvent;
        use rust_decimal_macros::dec;

        /// Helper to create a Level from i64 values
        fn level(price: i64, amount: i64) -> Level {
            Level::new(Decimal::from(price), Decimal::from(amount))
        }

        #[test]
        fn test_orderbook_update_flow_with_sequencer() {
            // Simulate a full flow:
            // 1. Start with a snapshot (seq=100)
            // 2. Receive updates that build on the snapshot
            // 3. Apply updates to OrderBook

            // Initial snapshot-based OrderBook
            let mut book = OrderBook::new(
                100, // sequence from snapshot
                None,
                vec![
                    level(100, 10), // bid at 100 with amount 10
                    level(99, 20),  // bid at 99 with amount 20
                    level(98, 30),  // bid at 98 with amount 30
                ],
                vec![
                    level(101, 5),  // ask at 101 with amount 5
                    level(102, 10), // ask at 102 with amount 10
                    level(103, 15), // ask at 103 with amount 15
                ],
            );

            // Initialize sequencer with snapshot version
            let mut state: L2SequencerState<&str> = L2SequencerState::new_for_test("BTCUSDT", 100);

            // Update 1: Valid first update (fromVersion=95, toVersion=110)
            // This should sync and be applied
            let result = state.validate_update(95, 110);
            assert_eq!(result, L2ValidationResult::Valid);
            
            // Simulate update data: modify bid at 100, add new ask at 104
            let update1 = OrderBookEvent::Update(OrderBook::new(
                110,
                None,
                vec![level(100, 15)], // update bid at 100 to amount 15
                vec![level(104, 8)],  // add new ask at 104
            ));
            book.update(&update1);

            assert_eq!(book.sequence(), 110);
            // Check bid at 100 is now 15
            assert!(book.bids().levels().iter().any(|l| l.price == dec!(100) && l.amount == dec!(15)));
            // Check new ask at 104 exists
            assert!(book.asks().levels().iter().any(|l| l.price == dec!(104) && l.amount == dec!(8)));

            // Update 2: Valid subsequent update (fromVersion=111, toVersion=120)
            let result = state.validate_update(111, 120);
            assert_eq!(result, L2ValidationResult::Valid);

            // Simulate update: remove bid at 98 (amount=0), update ask at 101
            let update2 = OrderBookEvent::Update(OrderBook::new(
                120,
                None,
                vec![Level::new(dec!(98), dec!(0))], // remove bid at 98
                vec![level(101, 12)], // update ask at 101 to amount 12
            ));
            book.update(&update2);

            assert_eq!(book.sequence(), 120);
            // Check bid at 98 is removed
            assert!(!book.bids().levels().iter().any(|l| l.price == dec!(98)));
            // Check ask at 101 is now 12
            assert!(book.asks().levels().iter().any(|l| l.price == dec!(101) && l.amount == dec!(12)));

            // Update 3: Outdated update (toVersion <= last)
            let result = state.validate_update(100, 115);
            assert_eq!(result, L2ValidationResult::Outdated);
            // Book should remain unchanged
            assert_eq!(book.sequence(), 120);
        }

        #[test]
        fn test_orderbook_gap_triggers_resync() {
            // Simulate a gap detection scenario
            let mut book = OrderBook::new(
                100,
                None,
                vec![level(100, 10)],
                vec![level(101, 5)],
            );

            let mut state: L2SequencerState<&str> = L2SequencerState::new_for_test("BTCUSDT", 100);

            // First sync
            let result = state.validate_update(95, 110);
            assert_eq!(result, L2ValidationResult::Valid);

            let update1 = OrderBookEvent::Update(OrderBook::new(
                110,
                None,
                vec![level(100, 15)],
                vec![],
            ));
            book.update(&update1);

            // Gap detected - should trigger resync in real code
            let result = state.validate_update(150, 160);
            assert!(matches!(result, L2ValidationResult::Gap { .. }));

            // In real code, this would trigger a reconnection
            // The book should NOT be updated with this message
            assert_eq!(book.sequence(), 110);
        }

        #[test]
        fn test_orderbook_first_update_gap() {
            // Test gap detection on first update (snapshot too old)
            let book = OrderBook::new(
                100,
                None,
                vec![level(100, 10)],
                vec![level(101, 5)],
            );

            let mut state: L2SequencerState<&str> = L2SequencerState::new_for_test("BTCUSDT", 100);

            // First update has gap: fromVersion(150) > snapshot(100)
            let result = state.validate_update(150, 160);
            assert_eq!(
                result,
                L2ValidationResult::Gap {
                    prev_last_update_id: 100,
                    first_update_id: 150,
                }
            );

            // Book should remain at snapshot state
            assert_eq!(book.sequence(), 100);
        }

        #[test]
        fn test_orderbook_multiple_updates_then_gap() {
            // Test gap detection after several successful updates
            let mut book = OrderBook::new(
                100,
                None,
                vec![level(100, 10), level(99, 20)],
                vec![level(101, 5), level(102, 10)],
            );

            let mut state: L2SequencerState<&str> = L2SequencerState::new_for_test("BTCUSDT", 100);

            // First sync
            assert_eq!(state.validate_update(95, 110), L2ValidationResult::Valid);
            book.update(&OrderBookEvent::Update(OrderBook::new(
                110,
                None,
                vec![level(100, 12)],
                vec![],
            )));

            // Second update
            assert_eq!(state.validate_update(111, 120), L2ValidationResult::Valid);
            book.update(&OrderBookEvent::Update(OrderBook::new(
                120,
                None,
                vec![level(99, 25)],
                vec![],
            )));

            // Third update
            assert_eq!(state.validate_update(121, 130), L2ValidationResult::Valid);
            book.update(&OrderBookEvent::Update(OrderBook::new(
                130,
                None,
                vec![],
                vec![level(101, 8)],
            )));

            assert_eq!(book.sequence(), 130);

            // Now a gap
            let result = state.validate_update(200, 210);
            assert_eq!(
                result,
                L2ValidationResult::Gap {
                    prev_last_update_id: 130,
                    first_update_id: 200,
                }
            );

            // Book unchanged
            assert_eq!(book.sequence(), 130);
        }

        #[test]
        fn test_orderbook_level_removal() {
            // Test that zero-amount levels are removed
            let mut book = OrderBook::new(
                100,
                None,
                vec![
                    level(100, 10),
                    level(99, 20),
                    level(98, 30),
                ],
                vec![
                    level(101, 5),
                    level(102, 10),
                ],
            );

            let mut state: L2SequencerState<&str> = L2SequencerState::new_for_test("BTCUSDT", 100);

            // First sync and remove a level
            assert_eq!(state.validate_update(95, 110), L2ValidationResult::Valid);
            book.update(&OrderBookEvent::Update(OrderBook::new(
                110,
                None,
                vec![Level::new(dec!(99), dec!(0))], // Remove bid at 99
                vec![Level::new(dec!(101), dec!(0))], // Remove ask at 101
            )));

            assert_eq!(book.sequence(), 110);
            assert_eq!(book.bids().levels().len(), 2); // 100 and 98 remain
            assert_eq!(book.asks().levels().len(), 1); // only 102 remains
            assert!(!book.bids().levels().iter().any(|l| l.price == dec!(99)));
            assert!(!book.asks().levels().iter().any(|l| l.price == dec!(101)));
        }

        #[test]
        fn test_orderbook_level_insert_and_update() {
            // Test inserting new levels and updating existing ones
            let mut book = OrderBook::new(
                100,
                None,
                vec![level(100, 10)],
                vec![level(101, 5)],
            );

            let mut state: L2SequencerState<&str> = L2SequencerState::new_for_test("BTCUSDT", 100);

            // Sync with updates
            assert_eq!(state.validate_update(95, 110), L2ValidationResult::Valid);
            book.update(&OrderBookEvent::Update(OrderBook::new(
                110,
                None,
                vec![
                    level(100, 15),  // Update existing
                    level(99, 25),   // Insert new
                ],
                vec![
                    level(101, 8),   // Update existing
                    level(102, 12),  // Insert new
                ],
            )));

            assert_eq!(book.sequence(), 110);
            
            // Check bids
            assert!(book.bids().levels().iter().any(|l| l.price == dec!(100) && l.amount == dec!(15)));
            assert!(book.bids().levels().iter().any(|l| l.price == dec!(99) && l.amount == dec!(25)));
            
            // Check asks
            assert!(book.asks().levels().iter().any(|l| l.price == dec!(101) && l.amount == dec!(8)));
            assert!(book.asks().levels().iter().any(|l| l.price == dec!(102) && l.amount == dec!(12)));
        }
    }
}

