//! MEXC L2 OrderBook transformer with deferred snapshot fetching.
//!
//! This implementation follows MEXC's recommended approach for maintaining a local orderbook:
//!
//! 1. Connect to WebSocket and subscribe to depth updates
//! 2. Buffer incoming updates while fetching REST snapshot
//! 3. Align the snapshot with buffered updates using version validation:
//!    - If `toVersion < snapshot.version`: discard (outdated)
//!    - If `fromVersion > snapshot.version`: gap detected, refetch snapshot
//!    - If `fromVersion <= snapshot.version <= toVersion`: first valid update
//! 4. For subsequent updates: `fromVersion` must equal `prevToVersion + 1`
//! 5. Output snapshot + aligned updates only after successful validation
//!
//! See: <https://mexcdevelop.github.io/apidocs/spot_v3_en/>

use crate::{
    books::{Level, OrderBook},
    error::DataError,
    event::MarketEvent,
    exchange::mexc::{MexcSpot, market::extract_symbol_from_channel, proto::PushDataV3ApiWrapper},
    subscription::{
        Map,
        book::{OrderBookEvent, OrderBooksL2},
    },
    transformer::ExchangeTransformer,
};
use async_trait::async_trait;
use barter_instrument::exchange::ExchangeId;
use barter_integration::{
    Transformer, error::SocketError, protocol::websocket::WsMessage, subscription::SubscriptionId,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::Deserialize;
use std::collections::{HashMap, VecDeque};
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, warn};

/// REST API base URL for MEXC.
pub const MEXC_REST_BASE_URL: &str = "https://api.mexc.com";

/// Maximum buffer size to prevent unbounded memory growth.
const MAX_BUFFER_SIZE: usize = 1000;

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

    response.json().await.map_err(SocketError::Http)
}

/// Parsed update from WebSocket with version info.
#[derive(Debug, Clone)]
struct ParsedUpdate {
    from_version: u64,
    to_version: u64,
    timestamp: DateTime<Utc>,
    bids: Vec<Level>,
    asks: Vec<Level>,
}

impl ParsedUpdate {
    /// Try to parse a WebSocket message into a ParsedUpdate.
    fn try_from_ws_message(input: &PushDataV3ApiWrapper) -> Option<Self> {
        let depth = input.public_aggre_depths.as_ref()?;

        let parse_level = |l: &crate::exchange::mexc::proto::PublicAggreDepthV3ApiItem| {
            let price: Decimal = l.price.parse().ok()?;
            let amount: Decimal = l.quantity.parse().ok()?;
            Some(Level::new(price, amount))
        };

        Some(Self {
            from_version: depth.from_version.parse().ok()?,
            to_version: depth.to_version.parse().ok()?,
            timestamp: input
                .send_time
                .or(input.create_time)
                .and_then(DateTime::from_timestamp_millis)
                .unwrap_or_else(Utc::now),
            bids: depth.bids.iter().filter_map(parse_level).collect(),
            asks: depth.asks.iter().filter_map(parse_level).collect(),
        })
    }

    /// Convert this update into a MarketEvent.
    fn into_market_event<InstrumentKey: Clone>(
        self,
        instrument_key: &InstrumentKey,
    ) -> MarketEvent<InstrumentKey, OrderBookEvent> {
        MarketEvent {
            time_exchange: self.timestamp,
            time_received: Utc::now(),
            exchange: ExchangeId::Mexc,
            instrument: instrument_key.clone(),
            kind: OrderBookEvent::Update(OrderBook::new(
                self.to_version,
                Some(self.timestamp),
                self.bids,
                self.asks,
            )),
        }
    }
}

/// State for syncing a single symbol's orderbook.
#[derive(Debug)]
enum SyncState<InstrumentKey> {
    /// Waiting for snapshot to be fetched.
    /// Buffering updates until snapshot arrives.
    WaitingForSnapshot {
        instrument_key: InstrumentKey,
        symbol: String,
        snapshot_depth: u32,
        buffer: VecDeque<ParsedUpdate>,
        snapshot_rx: Option<oneshot::Receiver<Result<MexcRestSnapshot, SocketError>>>,
    },
    /// Snapshot received, looking for the first update that aligns.
    Aligning {
        instrument_key: InstrumentKey,
        symbol: String,
        snapshot_depth: u32,
        snapshot: MexcRestSnapshot,
        buffer: VecDeque<ParsedUpdate>,
    },
    /// Successfully synced, normal operation.
    Synced {
        instrument_key: InstrumentKey,
        last_to_version: u64,
    },
}

/// MEXC L2 orderbook transformer with deferred snapshot fetching.
///
/// This transformer implements MEXC's recommended orderbook maintenance approach:
/// 1. Buffer WebSocket updates while fetching REST snapshot
/// 2. Align buffered updates with snapshot version
/// 3. Validate continuous sequence: fromVersion == prev_toVersion + 1
#[derive(Debug)]
pub struct MexcOrderBooksL2Transformer<InstrumentKey> {
    /// Map from subscription ID to sync state.
    states: HashMap<SubscriptionId, SyncState<InstrumentKey>>,
}

#[async_trait]
impl<InstrumentKey> ExchangeTransformer<MexcSpot, InstrumentKey, OrderBooksL2>
    for MexcOrderBooksL2Transformer<InstrumentKey>
where
    InstrumentKey: Clone + PartialEq + Send + Sync,
{
    async fn init(
        instrument_map: Map<InstrumentKey>,
        _initial_snapshots: &[MarketEvent<InstrumentKey, OrderBookEvent>],
        _ws_sink_tx: mpsc::UnboundedSender<WsMessage>,
    ) -> Result<Self, DataError> {
        // Build initial state map - all symbols start in WaitingForSnapshot
        let mut states = HashMap::with_capacity(instrument_map.0.len());

        for (sub_id, instrument_key) in instrument_map.0.into_iter() {
            // Extract symbol from subscription ID
            // Format: "aggre.depth|BTCUSDT"
            let symbol = sub_id
                .as_ref()
                .split('|')
                .nth(1)
                .ok_or_else(|| {
                    DataError::Socket(format!(
                        "Invalid MEXC L2 subscription ID format: expected 'aggre.depth|SYMBOL', got '{}'",
                        sub_id.as_ref()
                    ))
                })?
                .to_string();

            if symbol.is_empty() {
                return Err(DataError::Socket(format!(
                    "Invalid MEXC L2 subscription ID: symbol is empty in '{}'",
                    sub_id.as_ref()
                )));
            }

            let state = SyncState::WaitingForSnapshot {
                instrument_key,
                symbol,
                // Note: snapshot_depth will be updated when we receive the first update
                // from the subscription context. For now, use default.
                snapshot_depth: 500,
                buffer: VecDeque::with_capacity(64),
                snapshot_rx: None,
            };
            states.insert(sub_id, state);
        }

        Ok(Self { states })
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
        let Some(symbol) = extract_symbol_from_channel(&input.channel) else {
            return vec![];
        };

        let sub_id = SubscriptionId::from(format!("aggre.depth|{}", symbol));
        if !self.states.contains_key(&sub_id) {
            return vec![];
        }

        let Some(update) = ParsedUpdate::try_from_ws_message(&input) else {
            return vec![];
        };

        self.process_update(&sub_id, update)
    }
}

impl<InstrumentKey> MexcOrderBooksL2Transformer<InstrumentKey>
where
    InstrumentKey: Clone,
{
    /// Process an update based on current state.
    fn process_update(
        &mut self,
        sub_id: &SubscriptionId,
        update: ParsedUpdate,
    ) -> Vec<Result<MarketEvent<InstrumentKey, OrderBookEvent>, DataError>> {
        let Some(state) = self.states.remove(sub_id) else {
            return vec![];
        };

        let (new_state, results) = match state {
            SyncState::WaitingForSnapshot {
                instrument_key,
                symbol,
                snapshot_depth,
                mut buffer,
                snapshot_rx,
            } => {
                buffer.push_back(update);

                // Limit buffer size to prevent memory issues
                while buffer.len() > MAX_BUFFER_SIZE {
                    buffer.pop_front();
                }

                // Check if snapshot is ready (if we have a receiver)
                let snapshot_rx = match snapshot_rx {
                    Some(mut rx) => match rx.try_recv() {
                        Ok(Ok(snapshot)) => {
                            debug!(
                                %symbol,
                                snapshot_version = snapshot.last_update_id,
                                buffer_size = buffer.len(),
                                "Snapshot received, transitioning to Aligning"
                            );
                            return self.finish_state_transition(
                                sub_id,
                                SyncState::Aligning {
                                    instrument_key,
                                    symbol,
                                    snapshot_depth,
                                    snapshot,
                                    buffer,
                                },
                                vec![],
                            );
                        }
                        Ok(Err(error)) => {
                            warn!(%symbol, ?error, "Failed to fetch snapshot, will retry");
                            None // Reset - will trigger new snapshot request
                        }
                        Err(oneshot::error::TryRecvError::Empty) => Some(rx),
                        Err(oneshot::error::TryRecvError::Closed) => {
                            warn!(%symbol, "Snapshot channel closed unexpectedly");
                            None // Reset - will trigger new snapshot request
                        }
                    },
                    None => {
                        // No snapshot request yet - trigger one now
                        debug!(%symbol, depth = snapshot_depth, "Triggering snapshot fetch");
                        let (tx, rx) = oneshot::channel();
                        let symbol_clone = symbol.clone();
                        tokio::spawn(async move {
                            let _ = tx.send(fetch_snapshot(&symbol_clone, snapshot_depth).await);
                        });
                        Some(rx)
                    }
                };

                (
                    SyncState::WaitingForSnapshot {
                        instrument_key,
                        symbol,
                        snapshot_depth,
                        buffer,
                        snapshot_rx,
                    },
                    vec![],
                )
            }

            SyncState::Aligning {
                instrument_key,
                symbol,
                snapshot_depth,
                snapshot,
                mut buffer,
            } => {
                buffer.push_back(update);

                while buffer.len() > MAX_BUFFER_SIZE {
                    buffer.pop_front();
                }

                let snapshot_version = snapshot.last_update_id;

                // Find first update where fromVersion <= snapshot.version <= toVersion
                let aligned_idx = buffer.iter().position(|u| {
                    u.from_version <= snapshot_version && snapshot_version <= u.to_version
                });

                match aligned_idx {
                    Some(idx) => {
                        let snapshot_ts = snapshot
                            .timestamp
                            .and_then(|ms| DateTime::from_timestamp_millis(ms as i64))
                            .unwrap_or_else(Utc::now);

                        let parse_levels = |levels: &[[String; 2]]| -> Vec<Level> {
                            levels
                                .iter()
                                .filter_map(|[price, amount]| {
                                    let p: Decimal = price.parse().ok()?;
                                    let a: Decimal = amount.parse().ok()?;
                                    Some(Level::new(p, a))
                                })
                                .collect()
                        };

                        let mut results = vec![Ok(MarketEvent {
                            time_exchange: snapshot_ts,
                            time_received: Utc::now(),
                            exchange: ExchangeId::Mexc,
                            instrument: instrument_key.clone(),
                            kind: OrderBookEvent::Snapshot(OrderBook::new(
                                snapshot_version,
                                Some(snapshot_ts),
                                parse_levels(&snapshot.bids),
                                parse_levels(&snapshot.asks),
                            )),
                        })];

                        debug!(
                            %symbol,
                            snapshot_version,
                            aligned_update_idx = idx,
                            "L2 stream synced with snapshot"
                        );

                        // Discard updates before the aligned one
                        buffer.drain(..idx);

                        // Output all buffered updates from aligned one onwards
                        let mut last_to_version = snapshot_version;
                        for u in buffer.drain(..) {
                            // Validate sequence (allow first aligned update to have fromVersion <= snapshot_version)
                            if results.len() > 1 && u.from_version != last_to_version + 1 {
                                warn!(
                                    %symbol,
                                    expected = last_to_version + 1,
                                    actual = u.from_version,
                                    "Gap in buffered updates"
                                );
                                return self.finish_state_transition(
                                    sub_id,
                                    SyncState::Synced {
                                        instrument_key,
                                        last_to_version,
                                    },
                                    vec![Err(DataError::InvalidSequence {
                                        prev_last_update_id: last_to_version,
                                        first_update_id: u.from_version,
                                    })],
                                );
                            }
                            last_to_version = u.to_version;
                            results.push(Ok(u.into_market_event(&instrument_key)));
                        }

                        (
                            SyncState::Synced {
                                instrument_key,
                                last_to_version,
                            },
                            results,
                        )
                    }
                    None => {
                        // Check if all buffered updates are outdated
                        if !buffer.is_empty()
                            && buffer.iter().all(|u| u.to_version < snapshot_version)
                        {
                            debug!(
                                %symbol,
                                snapshot_version,
                                buffered_count = buffer.len(),
                                "All buffered updates outdated, clearing buffer"
                            );
                            buffer.clear();
                        }

                        // Check for gap (fromVersion > snapshot.version for all updates)
                        if let Some(earliest) = buffer.iter().map(|u| u.from_version).min()
                            && earliest > snapshot_version
                        {
                            warn!(%symbol, snapshot_version, earliest_from = earliest, "Snapshot too old, refetching");
                            return self.finish_state_transition(
                                sub_id,
                                SyncState::WaitingForSnapshot {
                                    instrument_key,
                                    symbol,
                                    snapshot_depth,
                                    buffer,
                                    snapshot_rx: None,
                                },
                                vec![],
                            );
                        }

                        (
                            SyncState::Aligning {
                                instrument_key,
                                symbol,
                                snapshot_depth,
                                snapshot,
                                buffer,
                            },
                            vec![],
                        )
                    }
                }
            }

            SyncState::Synced {
                instrument_key,
                last_to_version,
            } => {
                // Normal operation - validate sequence (fromVersion must equal prevToVersion + 1)
                if update.from_version != last_to_version + 1 {
                    (
                        SyncState::Synced {
                            instrument_key,
                            last_to_version,
                        },
                        vec![Err(DataError::InvalidSequence {
                            prev_last_update_id: last_to_version,
                            first_update_id: update.from_version,
                        })],
                    )
                } else {
                    let new_version = update.to_version;
                    let event = update.into_market_event(&instrument_key);
                    (
                        SyncState::Synced {
                            instrument_key,
                            last_to_version: new_version,
                        },
                        vec![Ok(event)],
                    )
                }
            }
        };

        self.states.insert(sub_id.clone(), new_state);
        results
    }

    /// Helper for early returns that need to insert state before returning.
    #[inline]
    fn finish_state_transition(
        &mut self,
        sub_id: &SubscriptionId,
        state: SyncState<InstrumentKey>,
        results: Vec<Result<MarketEvent<InstrumentKey, OrderBookEvent>, DataError>>,
    ) -> Vec<Result<MarketEvent<InstrumentKey, OrderBookEvent>, DataError>> {
        self.states.insert(sub_id.clone(), state);
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod parsed_update {
        use super::*;
        use crate::exchange::mexc::proto::{
            PublicAggreDepthV3ApiItem, PublicAggreDepthsV3Api, PushDataV3ApiWrapper,
        };

        #[test]
        fn test_try_from_ws_message_valid() {
            let input = PushDataV3ApiWrapper {
                channel: "spot@public.aggre.depth.v3.api.pb@100ms@BTCUSDT".to_string(),
                send_time: Some(1700000000000),
                create_time: None,
                public_aggre_depths: Some(PublicAggreDepthsV3Api {
                    from_version: "100".to_string(),
                    to_version: "110".to_string(),
                    event_type: "depth".to_string(),
                    bids: vec![PublicAggreDepthV3ApiItem {
                        price: "50000.00".to_string(),
                        quantity: "1.5".to_string(),
                    }],
                    asks: vec![PublicAggreDepthV3ApiItem {
                        price: "50001.00".to_string(),
                        quantity: "0.5".to_string(),
                    }],
                }),
                public_limit_depths: None,
                public_increase_depths: None,
                symbol: None,
                symbol_id: None,
            };

            let update = ParsedUpdate::try_from_ws_message(&input).unwrap();
            assert_eq!(update.from_version, 100);
            assert_eq!(update.to_version, 110);
            assert_eq!(update.bids.len(), 1);
            assert_eq!(update.asks.len(), 1);
        }

        #[test]
        fn test_try_from_ws_message_no_depth() {
            let input = PushDataV3ApiWrapper {
                channel: "spot@public.aggre.depth.v3.api.pb@100ms@BTCUSDT".to_string(),
                send_time: Some(1700000000000),
                create_time: None,
                public_aggre_depths: None,
                public_limit_depths: None,
                public_increase_depths: None,
                symbol: None,
                symbol_id: None,
            };

            assert!(ParsedUpdate::try_from_ws_message(&input).is_none());
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

    mod sync_state {
        use super::*;
        use rust_decimal_macros::dec;

        fn make_update(from: u64, to: u64) -> ParsedUpdate {
            ParsedUpdate {
                from_version: from,
                to_version: to,
                timestamp: Utc::now(),
                bids: vec![Level::new(dec!(100), dec!(1))],
                asks: vec![Level::new(dec!(101), dec!(1))],
            }
        }

        fn make_snapshot(version: u64) -> MexcRestSnapshot {
            MexcRestSnapshot {
                last_update_id: version,
                bids: vec![["100".to_string(), "1".to_string()]],
                asks: vec![["101".to_string(), "1".to_string()]],
                timestamp: Some(1700000000000),
            }
        }

        #[test]
        fn test_alignment_exact_match() {
            // Snapshot version 100, update range [100, 110]
            let snapshot = make_snapshot(100);
            let update = make_update(100, 110);

            // fromVersion(100) <= snapshot(100) <= toVersion(110) -> should align
            assert!(update.from_version <= snapshot.last_update_id);
            assert!(snapshot.last_update_id <= update.to_version);
        }

        #[test]
        fn test_alignment_with_overlap() {
            // Snapshot version 100, update range [90, 110]
            let snapshot = make_snapshot(100);
            let update = make_update(90, 110);

            // fromVersion(90) <= snapshot(100) <= toVersion(110) -> should align
            assert!(update.from_version <= snapshot.last_update_id);
            assert!(snapshot.last_update_id <= update.to_version);
        }

        #[test]
        fn test_update_outdated() {
            // Snapshot version 100, update range [80, 90]
            let snapshot = make_snapshot(100);
            let update = make_update(80, 90);

            // toVersion(90) < snapshot(100) -> outdated
            assert!(update.to_version < snapshot.last_update_id);
        }

        #[test]
        fn test_snapshot_too_old() {
            // Snapshot version 100, update range [110, 120]
            let snapshot = make_snapshot(100);
            let update = make_update(110, 120);

            // fromVersion(110) > snapshot(100) -> gap, snapshot too old
            assert!(update.from_version > snapshot.last_update_id);
        }

        #[test]
        fn test_subsequent_update_valid() {
            // After syncing with toVersion 110
            // Next update should have fromVersion 111
            let last_to_version = 110u64;
            let update = make_update(111, 120);

            assert_eq!(update.from_version, last_to_version + 1);
        }

        #[test]
        fn test_subsequent_update_gap() {
            // After syncing with toVersion 110
            // Update with fromVersion 115 is a gap
            let last_to_version = 110u64;
            let update = make_update(115, 125);

            assert_ne!(update.from_version, last_to_version + 1);
        }
    }

    mod alignment {
        use super::*;
        use rust_decimal_macros::dec;

        fn make_update(from: u64, to: u64) -> ParsedUpdate {
            ParsedUpdate {
                from_version: from,
                to_version: to,
                timestamp: Utc::now(),
                bids: vec![Level::new(dec!(100), dec!(1))],
                asks: vec![Level::new(dec!(101), dec!(1))],
            }
        }

        #[test]
        fn test_alignment_finds_correct_update() {
            let updates = vec![
                make_update(80, 90),   // outdated
                make_update(91, 100),  // outdated
                make_update(101, 110), // alignment point for snapshot=105
                make_update(111, 120), // valid next
            ];

            let snapshot_version = 105u64;

            let alignment_idx = updates.iter().position(|update| {
                update.from_version <= snapshot_version && snapshot_version <= update.to_version
            });

            assert_eq!(alignment_idx, Some(2));
        }

        #[test]
        fn test_alignment_no_match_all_outdated() {
            let updates = vec![make_update(80, 90), make_update(91, 100)];

            let snapshot_version = 150u64;

            let alignment_idx = updates.iter().position(|update| {
                update.from_version <= snapshot_version && snapshot_version <= update.to_version
            });

            assert_eq!(alignment_idx, None);
        }

        #[test]
        fn test_alignment_no_match_gap() {
            let updates = vec![
                make_update(200, 210), // gap - snapshot too old
            ];

            let snapshot_version = 100u64;

            let alignment_idx = updates.iter().position(|update| {
                update.from_version <= snapshot_version && snapshot_version <= update.to_version
            });

            assert_eq!(alignment_idx, None);

            // Verify this is a gap scenario (first update is after snapshot)
            assert!(updates[0].from_version > snapshot_version);
        }

        #[test]
        fn test_sequence_validation_continuous() {
            let updates = vec![
                make_update(100, 110), // alignment point for snapshot=105
                make_update(111, 120), // valid: fromVersion == prev_toVersion + 1
                make_update(121, 130), // valid: fromVersion == prev_toVersion + 1
            ];

            let snapshot_version = 105u64;
            let mut last_to_version = snapshot_version;
            let mut valid_count = 0;

            for (i, update) in updates.iter().enumerate() {
                // Skip first (alignment) - it's always valid if found
                if i == 0 {
                    last_to_version = update.to_version;
                    valid_count += 1;
                    continue;
                }

                if update.from_version == last_to_version + 1 {
                    last_to_version = update.to_version;
                    valid_count += 1;
                } else {
                    break;
                }
            }

            assert_eq!(valid_count, 3);
            assert_eq!(last_to_version, 130);
        }

        #[test]
        fn test_sequence_validation_with_gap() {
            let updates = vec![
                make_update(100, 110), // alignment point
                make_update(111, 120), // valid
                make_update(125, 130), // GAP: expected 121, got 125
                make_update(131, 140), // won't be processed
            ];

            let snapshot_version = 105u64;
            let mut last_to_version = snapshot_version;
            let mut valid_count = 0;

            for (i, update) in updates.iter().enumerate() {
                if i == 0 {
                    last_to_version = update.to_version;
                    valid_count += 1;
                    continue;
                }

                if update.from_version == last_to_version + 1 {
                    last_to_version = update.to_version;
                    valid_count += 1;
                } else {
                    break;
                }
            }

            assert_eq!(valid_count, 2); // Only first 2 are valid
            assert_eq!(last_to_version, 120);
        }
    }

    /// Integration tests for the full MEXC L2 orderbook flow.
    /// These tests require network access and validate the complete snapshot + update stream.
    ///
    /// Run with: `cargo test -p barter-data --lib mexc::book::l2::tests::integration -- --ignored`
    mod integration {
        use super::*;
        use crate::{
            books::OrderBook,
            exchange::mexc::MexcSpot,
            streams::{
                Streams,
                reconnect::{Event, stream::ReconnectingStream},
            },
            subscription::book::OrderBooksL2,
        };
        use barter_instrument::instrument::market_data::kind::MarketDataInstrumentKind;
        use futures_util::StreamExt;
        use rust_decimal::Decimal;
        use std::time::Duration;
        use tokio::time::timeout;

        /// Full integration test that validates the MEXC L2 orderbook implementation:
        ///
        /// 1. Subscribes to MEXC WebSocket for L2 orderbook updates
        /// 2. Validates that the transformer correctly buffers updates
        /// 3. Validates that snapshot is fetched and aligned with buffered updates
        /// 4. Validates that snapshot is emitted first, followed by aligned updates
        /// 5. Validates that subsequent updates follow continuous sequence rule:
        ///    `fromVersion == prev_toVersion + 1`
        /// 6. Validates orderbook integrity after applying updates
        ///
        /// This test validates the fix for the MEXC orderbook maintenance approach as
        /// documented at: https://mexcdevelop.github.io/apidocs/spot_v3_en/
        #[tokio::test]
        #[ignore] // Requires network access - run with: cargo test --lib -- --ignored
        async fn test_full_snapshot_and_update_stream() {
            // Initialize tracing for debugging
            let _ = tracing_subscriber::fmt()
                .with_env_filter("barter_data=debug,multi_stream=info")
                .try_init();

            // Subscribe to MEXC BTC/USDT L2 orderbook
            let streams = Streams::<OrderBooksL2>::builder()
                .subscribe([(
                    MexcSpot::default(),
                    "btc",
                    "usdt",
                    MarketDataInstrumentKind::Spot,
                    OrderBooksL2,
                )])
                .init()
                .await
                .expect("Failed to initialize MEXC L2 stream");

            let mut stream = streams.select_all().with_error_handler(|error| {
                panic!("MarketStream error during test: {:?}", error);
            });

            // Track state for validation
            let mut snapshot_count: u32 = 0;
            let mut snapshot_sequence: u64 = 0;
            let mut last_update_sequence: u64 = 0;
            let mut received_updates: u32 = 0;
            let mut local_book = OrderBook::default();
            let mut reconnect_count = 0;

            const REQUIRED_UPDATES: u32 = 3; // Validate at least 3 updates after snapshot

            // Run test with timeout
            let test_result = timeout(Duration::from_secs(30), async {
                while let Some(event) = stream.next().await {
                    match event {
                        Event::Reconnecting(exchange) => {
                            reconnect_count += 1;
                            panic!(
                                "Unexpected reconnection to {:?} - this indicates a sequence gap!",
                                exchange
                            );
                        }
                        Event::Item(market_event) => {
                            match &market_event.kind {
                                OrderBookEvent::Snapshot(book) => {
                                    snapshot_count += 1;

                                    // Validate we only receive ONE snapshot - fail immediately
                                    assert_eq!(
                                        snapshot_count, 1,
                                        "Received {} snapshots - should only receive exactly 1!",
                                        snapshot_count
                                    );

                                    snapshot_sequence = book.sequence();

                                    // Validate snapshot has data
                                    assert!(
                                        !book.bids().levels().is_empty(),
                                        "Snapshot should have bids"
                                    );
                                    assert!(
                                        !book.asks().levels().is_empty(),
                                        "Snapshot should have asks"
                                    );
                                    assert!(
                                        book.sequence() > 0,
                                        "Snapshot should have a sequence number"
                                    );

                                    // Initialize local book from snapshot
                                    local_book = book.clone();
                                    last_update_sequence = book.sequence();

                                    println!(
                                        "✓ Received snapshot: seq={}, bids={}, asks={}",
                                        book.sequence(),
                                        book.bids().levels().len(),
                                        book.asks().levels().len()
                                    );
                                }
                                OrderBookEvent::Update(update) => {
                                    // Validate updates only come after snapshot
                                    assert!(
                                        snapshot_count == 1,
                                        "Received update but snapshot_count={} (expected 1)",
                                        snapshot_count
                                    );

                                    // Validate sequence continuity
                                    if received_updates > 0 {
                                        assert!(
                                            update.sequence() > last_update_sequence,
                                            "Update sequence {} should be > last sequence {}",
                                            update.sequence(),
                                            last_update_sequence
                                        );
                                    }

                                    // Apply update to local book
                                    local_book.update(&market_event.kind);
                                    last_update_sequence = update.sequence();
                                    received_updates += 1;

                                    // Validate orderbook integrity after each update
                                    validate_orderbook_integrity(&local_book);

                                    println!(
                                        "✓ Applied update #{}: seq={}, Δbids={}, Δasks={}",
                                        received_updates,
                                        update.sequence(),
                                        update.bids().levels().len(),
                                        update.asks().levels().len()
                                    );

                                    // Once we've received enough updates, test passes
                                    if received_updates >= REQUIRED_UPDATES {
                                        println!(
                                            "\n✓ Successfully validated {} updates after snapshot",
                                            REQUIRED_UPDATES
                                        );
                                        return;
                                    }
                                }
                            }
                        }
                    }
                }
            })
            .await;

            // Final assertions
            assert!(test_result.is_ok(), "Test timed out after 30 seconds");
            assert_eq!(snapshot_count, 1, "Should have received exactly 1 snapshot");
            assert_eq!(reconnect_count, 0, "Should not have any reconnections");
            assert!(
                received_updates >= REQUIRED_UPDATES,
                "Should have received at least {} updates, got {}",
                REQUIRED_UPDATES,
                received_updates
            );

            // Validate final orderbook state
            validate_orderbook_integrity(&local_book);

            println!(
                "\n========================================\n\
                 INTEGRATION TEST PASSED\n\
                 ========================================\n\
                 • Snapshot count: {} (expected: 1)\n\
                 • Snapshot sequence: {}\n\
                 • Final sequence: {}\n\
                 • Updates validated: {}\n\
                 • Reconnections: {}\n\
                 • Final book: {} bids, {} asks\n\
                 ========================================",
                snapshot_count,
                snapshot_sequence,
                local_book.sequence(),
                received_updates,
                reconnect_count,
                local_book.bids().levels().len(),
                local_book.asks().levels().len()
            );
        }

        /// Validates that an OrderBook is in a consistent state.
        fn validate_orderbook_integrity(book: &OrderBook) {
            // Check that best bid < best ask (no crossed book)
            if let (Some(best_bid), Some(best_ask)) = (book.bids().best(), book.asks().best()) {
                assert!(
                    best_bid.price < best_ask.price,
                    "Orderbook is crossed! Best bid {} >= best ask {}",
                    best_bid.price,
                    best_ask.price
                );
            }

            // Check that bids are sorted descending (highest first)
            let bids = book.bids().levels();
            for window in bids.windows(2) {
                assert!(
                    window[0].price >= window[1].price,
                    "Bids not sorted descending: {} < {}",
                    window[0].price,
                    window[1].price
                );
            }

            // Check that asks are sorted ascending (lowest first)
            let asks = book.asks().levels();
            for window in asks.windows(2) {
                assert!(
                    window[0].price <= window[1].price,
                    "Asks not sorted ascending: {} > {}",
                    window[0].price,
                    window[1].price
                );
            }

            // Check that all amounts are positive
            for level in bids {
                assert!(
                    level.amount > Decimal::ZERO,
                    "Bid level has non-positive amount: {:?}",
                    level
                );
            }
            for level in asks {
                assert!(
                    level.amount > Decimal::ZERO,
                    "Ask level has non-positive amount: {:?}",
                    level
                );
            }
        }

        /// Test that validates multiple symbols can be subscribed simultaneously
        /// and each receives exactly one snapshot followed by updates.
        #[tokio::test]
        #[ignore] // Requires network access
        async fn test_multiple_symbols_snapshot_and_updates() {
            let _ = tracing_subscriber::fmt()
                .with_env_filter("barter_data=debug")
                .try_init();

            // Subscribe to both BTC/USDT and ETH/USDT
            let streams = Streams::<OrderBooksL2>::builder()
                .subscribe([
                    (
                        MexcSpot::default(),
                        "btc",
                        "usdt",
                        MarketDataInstrumentKind::Spot,
                        OrderBooksL2,
                    ),
                    (
                        MexcSpot::default(),
                        "eth",
                        "usdt",
                        MarketDataInstrumentKind::Spot,
                        OrderBooksL2,
                    ),
                ])
                .init()
                .await
                .expect("Failed to initialize MEXC L2 streams");

            let mut stream = streams.select_all().with_error_handler(|error| {
                panic!("MarketStream error: {:?}", error);
            });

            let mut btc_snapshot_count: u32 = 0;
            let mut eth_snapshot_count: u32 = 0;
            let mut btc_update_count: u32 = 0;
            let mut eth_update_count: u32 = 0;

            let test_result = timeout(Duration::from_secs(30), async {
                while let Some(event) = stream.next().await {
                    match event {
                        Event::Reconnecting(_) => {
                            panic!("Unexpected reconnection!");
                        }
                        Event::Item(market_event) => {
                            let instrument = format!("{:?}", market_event.instrument);
                            let is_btc = instrument.to_lowercase().contains("btc");
                            let is_eth = instrument.to_lowercase().contains("eth");

                            match &market_event.kind {
                                OrderBookEvent::Snapshot(_) => {
                                    if is_btc {
                                        btc_snapshot_count += 1;
                                        // Fail immediately if more than 1 snapshot for BTC
                                        assert_eq!(
                                            btc_snapshot_count, 1,
                                            "BTC received {} snapshots - should only receive 1!",
                                            btc_snapshot_count
                                        );
                                        println!("✓ BTC snapshot received (count: {})", btc_snapshot_count);
                                    } else if is_eth {
                                        eth_snapshot_count += 1;
                                        // Fail immediately if more than 1 snapshot for ETH
                                        assert_eq!(
                                            eth_snapshot_count, 1,
                                            "ETH received {} snapshots - should only receive 1!",
                                            eth_snapshot_count
                                        );
                                        println!("✓ ETH snapshot received (count: {})", eth_snapshot_count);
                                    }
                                }
                                OrderBookEvent::Update(_) => {
                                    if is_btc {
                                        // Ensure we got snapshot before updates
                                        assert_eq!(
                                            btc_snapshot_count, 1,
                                            "BTC update received but snapshot_count={} (expected 1)",
                                            btc_snapshot_count
                                        );
                                        btc_update_count += 1;
                                    } else if is_eth {
                                        // Ensure we got snapshot before updates
                                        assert_eq!(
                                            eth_snapshot_count, 1,
                                            "ETH update received but snapshot_count={} (expected 1)",
                                            eth_snapshot_count
                                        );
                                        eth_update_count += 1;
                                    }
                                }
                            }

                            // Test passes when both have exactly 1 snapshot + at least 2 updates
                            if btc_snapshot_count == 1
                                && eth_snapshot_count == 1
                                && btc_update_count >= 2
                                && eth_update_count >= 2
                            {
                                return;
                            }
                        }
                    }
                }
            })
            .await;

            assert!(test_result.is_ok(), "Test timed out");
            assert_eq!(
                btc_snapshot_count, 1,
                "BTC should have exactly 1 snapshot, got {}",
                btc_snapshot_count
            );
            assert_eq!(
                eth_snapshot_count, 1,
                "ETH should have exactly 1 snapshot, got {}",
                eth_snapshot_count
            );
            assert!(
                btc_update_count >= 2,
                "BTC should have at least 2 updates, got {}",
                btc_update_count
            );
            assert!(
                eth_update_count >= 2,
                "ETH should have at least 2 updates, got {}",
                eth_update_count
            );

            println!(
                "\n✓ Multi-symbol test passed:\n\
                   • BTC: {} snapshot(s), {} update(s)\n\
                   • ETH: {} snapshot(s), {} update(s)",
                btc_snapshot_count, btc_update_count, eth_snapshot_count, eth_update_count
            );
        }
    }
}
