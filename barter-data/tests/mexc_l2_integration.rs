//! MEXC L2 OrderBook integration tests.
//!
//! These tests require network access to MEXC's WebSocket and REST APIs.
//! Run with: `cargo test -p barter-data --test mexc_l2_integration -- --ignored`

use barter_data::{
    books::OrderBook,
    exchange::mexc::MexcSpot,
    streams::{
        Streams,
        reconnect::{Event, stream::ReconnectingStream},
    },
    subscription::book::{OrderBookEvent, OrderBooksL2},
};
use barter_instrument::instrument::market_data::kind::MarketDataInstrumentKind;
use futures_util::StreamExt;
use std::time::Duration;
use tokio::time::timeout;

/// Integration test that validates the full MEXC L2 subscription flow:
/// 1. Subscribes to MEXC L2 orderbook stream
/// 2. Validates that a snapshot is retrieved
/// 3. Validates that incremental updates are applied
/// 4. Validates that the orderbook is correctly constructed
#[tokio::test]
#[ignore] // Requires network access - run with: cargo test --test mexc_l2_integration -- --ignored
async fn test_mexc_l2_full_subscription_flow() {
    // Initialize tracing for debugging (optional)
    let _ = tracing_subscriber::fmt()
        .with_env_filter("barter_data=debug")
        .try_init();

    // Subscribe to MEXC ETH/USDT L2 orderbook with default depth (500)
    let streams = Streams::<OrderBooksL2>::builder()
        .subscribe([(
            MexcSpot::default(),
            "eth",
            "usdt",
            MarketDataInstrumentKind::Spot,
            OrderBooksL2,
        )])
        .init()
        .await
        .expect("Failed to initialize MEXC L2 stream");

    // Get the merged stream with error handling
    let mut stream = streams.select_all().with_error_handler(|error| {
        panic!("MarketStream error: {:?}", error);
    });

    // Track state
    let mut received_snapshot = false;
    let mut received_updates = 0;
    let mut local_book = OrderBook::default();
    const REQUIRED_UPDATES: usize = 5;

    // Set a timeout for the entire test
    let test_result = timeout(Duration::from_secs(30), async {
        while let Some(event) = stream.next().await {
            // Handle reconnection events
            let market_event = match event {
                Event::Item(item) => item,
                Event::Reconnecting(exchange) => {
                    println!("Reconnecting to {:?}...", exchange);
                    continue;
                }
            };

            match &market_event.kind {
                OrderBookEvent::Snapshot(book) => {
                    assert!(!received_snapshot, "Should only receive one snapshot");
                    received_snapshot = true;

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

                    println!(
                        "Received snapshot: seq={}, bids={}, asks={}",
                        book.sequence(),
                        book.bids().levels().len(),
                        book.asks().levels().len()
                    );
                }
                OrderBookEvent::Update(update) => {
                    assert!(received_snapshot, "Should receive snapshot before updates");

                    // Apply update to local book
                    local_book.update(&market_event.kind);
                    received_updates += 1;

                    println!(
                        "Applied update #{}: seq={}, bids_delta={}, asks_delta={}",
                        received_updates,
                        update.sequence(),
                        update.bids().levels().len(),
                        update.asks().levels().len()
                    );

                    // Validate book is still in good state
                    validate_orderbook(&local_book);

                    // Once we've received enough updates, we're done
                    if received_updates >= REQUIRED_UPDATES {
                        break;
                    }
                }
            }
        }
    })
    .await;

    // Validate results
    assert!(test_result.is_ok(), "Test timed out after 30 seconds");
    assert!(received_snapshot, "Should have received a snapshot");
    assert!(
        received_updates >= REQUIRED_UPDATES,
        "Should have received at least {} updates, got {}",
        REQUIRED_UPDATES,
        received_updates
    );

    // Final validation of the orderbook
    validate_orderbook(&local_book);

    println!(
        "\nTest passed! Final book state: seq={}, bids={}, asks={}",
        local_book.sequence(),
        local_book.bids().levels().len(),
        local_book.asks().levels().len()
    );
}

/// Integration test with custom snapshot depth
#[tokio::test]
#[ignore] // Requires network access
async fn test_mexc_l2_custom_snapshot_depth() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("barter_data=debug")
        .try_init();

    // Subscribe with custom snapshot depth of 100
    let streams = Streams::<OrderBooksL2>::builder()
        .subscribe([(
            MexcSpot::with_snapshot_depth(100),
            "btc",
            "usdt",
            MarketDataInstrumentKind::Spot,
            OrderBooksL2,
        )])
        .init()
        .await
        .expect("Failed to initialize MEXC L2 stream with custom depth");

    let mut stream = streams.select_all().with_error_handler(|error| {
        panic!("MarketStream error: {:?}", error);
    });

    // Wait for snapshot
    let test_result = timeout(Duration::from_secs(15), async {
        while let Some(event) = stream.next().await {
            let market_event = match event {
                Event::Item(item) => item,
                Event::Reconnecting(_) => continue,
            };

            if let OrderBookEvent::Snapshot(book) = &market_event.kind {
                // With depth=100, we should have at most 100 levels on each side
                assert!(
                    book.bids().levels().len() <= 100,
                    "Should have at most 100 bid levels with depth=100, got {}",
                    book.bids().levels().len()
                );
                assert!(
                    book.asks().levels().len() <= 100,
                    "Should have at most 100 ask levels with depth=100, got {}",
                    book.asks().levels().len()
                );

                println!(
                    "Custom depth snapshot: bids={}, asks={}",
                    book.bids().levels().len(),
                    book.asks().levels().len()
                );
                return;
            }
        }
    })
    .await;

    assert!(test_result.is_ok(), "Test timed out waiting for snapshot");
}

/// Integration test for multiple symbols
#[tokio::test]
#[ignore] // Requires network access
async fn test_mexc_l2_multiple_symbols() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("barter_data=debug")
        .try_init();

    // Subscribe to multiple symbols
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
        .expect("Failed to initialize MEXC L2 streams for multiple symbols");

    let mut stream = streams.select_all().with_error_handler(|error| {
        panic!("MarketStream error: {:?}", error);
    });

    let mut btc_snapshot = false;
    let mut eth_snapshot = false;

    let test_result = timeout(Duration::from_secs(20), async {
        while let Some(event) = stream.next().await {
            let market_event = match event {
                Event::Item(item) => item,
                Event::Reconnecting(_) => continue,
            };

            if let OrderBookEvent::Snapshot(_) = &market_event.kind {
                // Check which instrument this is for
                let instrument = format!("{:?}", market_event.instrument);
                if instrument.contains("btc") {
                    btc_snapshot = true;
                    println!("Received BTC/USDT snapshot");
                } else if instrument.contains("eth") {
                    eth_snapshot = true;
                    println!("Received ETH/USDT snapshot");
                }

                if btc_snapshot && eth_snapshot {
                    break;
                }
            }
        }
    })
    .await;

    assert!(test_result.is_ok(), "Test timed out");
    assert!(btc_snapshot, "Should have received BTC/USDT snapshot");
    assert!(eth_snapshot, "Should have received ETH/USDT snapshot");
}

/// Validates that an OrderBook is in a consistent state
fn validate_orderbook(book: &OrderBook) {
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
            level.amount > rust_decimal::Decimal::ZERO,
            "Bid level has non-positive amount: {:?}",
            level
        );
    }
    for level in asks {
        assert!(
            level.amount > rust_decimal::Decimal::ZERO,
            "Ask level has non-positive amount: {:?}",
            level
        );
    }
}
