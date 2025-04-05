use crate::{EngineEvent, engine::Processor, execution::AccountStreamEvent};
use barter_data::streams::consumer::MarketStreamEvent;
use barter_execution::AccountEventKind;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, ops::Add, sync::Arc};
use tracing::{debug, error, warn};

/// Defines how an [`Engine`](super::Engine) will determine the current time.
///
/// Generally an `Engine` will use a:
/// * [`LiveClock`] for live-trading.
/// * [`HistoricalClock`] for back-testing.
pub trait EngineClock {
    fn time(&self) -> DateTime<Utc>;
}

/// Defines how to extract an "exchange timestamp" from an event.
///
/// Used by a [`HistoricalClock`] to assist deriving the "current" `Engine` time.
pub trait TimeExchange {
    fn time_exchange(&self) -> Option<DateTime<Utc>>;
}

/// Live `Clock` using `Utc::now()`.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct LiveClock;

impl EngineClock for LiveClock {
    fn time(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

impl<Event> Processor<&Event> for LiveClock {
    type Audit = ();

    fn process(&mut self, _: &Event) -> Self::Audit {}
}

/// Historical `Clock` using processed event timestamps to estimate current historical time.
///
/// Note that this cannot be initialised without a starting `last_exchange_timestamp`.
#[derive(Debug, Clone)]
pub struct HistoricalClock {
    inner: Arc<parking_lot::RwLock<HistoricalClockInner>>,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
struct HistoricalClockInner {
    time_exchange_last: DateTime<Utc>,
    time_live_last_event: DateTime<Utc>,
}

impl HistoricalClock {
    /// Construct a new `HistoricalClock` using the provided `last_exchange_time` as a seed.
    pub fn new(last_exchange_time: DateTime<Utc>) -> Self {
        Self {
            inner: Arc::new(parking_lot::RwLock::new(HistoricalClockInner {
                time_exchange_last: last_exchange_time,
                time_live_last_event: Utc::now(),
            })),
        }
    }
}

impl EngineClock for HistoricalClock {
    fn time(&self) -> DateTime<Utc> {
        let lock = self.inner.read();
        let time_live_last_event = lock.time_live_last_event;
        let time_exchange_last = lock.time_exchange_last;
        drop(lock);

        let delta_since_last_event_live_time =
            Utc::now().signed_duration_since(time_live_last_event);

        // Edge case: only add TimeDelta if it's positive to handle out of order updates
        match delta_since_last_event_live_time {
            delta if delta.num_milliseconds() >= 0 => time_exchange_last.add(delta),
            _ => time_exchange_last,
        }
    }
}

impl<Event> Processor<&Event> for HistoricalClock
where
    Event: Debug + TimeExchange,
{
    type Audit = ();

    fn process(&mut self, event: &Event) -> Self::Audit {
        let Some(time_event_exchange) = event.time_exchange() else {
            debug!(?event, "HistoricalClock found no timestamp in event");
            return;
        };

        // Obtain lock
        let mut lock = self.inner.write();

        // Input event is more recent
        if time_event_exchange >= lock.time_exchange_last {
            debug!(
                ?event,
                time_exchange_last_current = ?lock.time_exchange_last,
                time_update = ?time_event_exchange,
                "HistoricalClock updating based on input event time_exchange"
            );
            lock.time_exchange_last = time_event_exchange;
            lock.time_live_last_event = Utc::now();
            return;
        };

        // Input event is older, so log at varying degrees of severity
        let time_diff_secs = time_event_exchange
            .signed_duration_since(lock.time_exchange_last)
            .num_seconds()
            .abs();

        if time_diff_secs < 1 {
            debug!(
                ?event,
                time_exchange_last_current = ?lock.time_exchange_last,
                time_update = ?time_event_exchange,
                time_diff_secs,
                "HistoricalClock received out-of-order events"
            );
        } else if time_diff_secs < 30 {
            warn!(
                ?event,
                time_exchange_last_current = ?lock.time_exchange_last,
                time_update = ?time_event_exchange,
                time_diff_secs,
                "HistoricalClock received out-of-order events"
            );
        } else {
            error!(
                ?event,
                time_exchange_last_current = ?lock.time_exchange_last,
                time_update = ?time_event_exchange,
                time_diff_secs,
                "HistoricalClock received out-of-order events"
            );
        }
    }
}

impl<MarketEventKind: Debug> TimeExchange for EngineEvent<MarketEventKind> {
    fn time_exchange(&self) -> Option<DateTime<Utc>> {
        match self {
            Self::Market(MarketStreamEvent::Item(event)) => Some(event.time_exchange),
            Self::Account(AccountStreamEvent::Item(event)) => match &event.kind {
                AccountEventKind::Snapshot(snapshot) => snapshot.time_most_recent(),
                AccountEventKind::BalanceSnapshot(balance) => Some(balance.0.time_exchange),
                AccountEventKind::OrderSnapshot(order) => order.0.state.time_exchange(),
                AccountEventKind::OrderCancelled(response) => response
                    .state
                    .as_ref()
                    .map(|cancelled| cancelled.time_exchange)
                    .ok(),
                AccountEventKind::Trade(trade) => Some(trade.time_exchange),
            },
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use barter_data::event::MarketEvent;
    use barter_instrument::{exchange::ExchangeId, instrument::InstrumentIndex};
    use chrono::TimeDelta;

    fn market_event(time_exchange: DateTime<Utc>) -> EngineEvent<()> {
        EngineEvent::Market(MarketStreamEvent::Item(MarketEvent {
            time_exchange,
            time_received: Default::default(),
            exchange: ExchangeId::BinanceSpot,
            instrument: InstrumentIndex::new(0),
            kind: (),
        }))
    }

    #[test]
    fn test_historical_clock_process() {
        #[derive(Debug)]
        struct TestCase {
            name: &'static str,
            time_initial: DateTime<Utc>,
            input_events: Vec<EngineEvent<()>>,
            expected_time_exchange_last: DateTime<Utc>,
            delay_ms: Option<u64>,
        }

        // Create a fixed initial time to use as a base
        let time_base = DateTime::<Utc>::MIN_UTC;

        // Util for adding time
        let plus_ms = |ms: i64| {
            time_base
                .checked_add_signed(TimeDelta::milliseconds(ms))
                .unwrap()
        };

        let cases = vec![
            // TC0: Basic case - single event in order
            TestCase {
                name: "single event in order",
                time_initial: time_base,
                input_events: vec![market_event(plus_ms(1000))],
                expected_time_exchange_last: plus_ms(1000),
                delay_ms: None,
            },
            // TC1: Out of order event - earlier than current
            TestCase {
                name: "out of order event - earlier than current",
                time_initial: plus_ms(1000),
                input_events: vec![market_event(plus_ms(500))],
                expected_time_exchange_last: plus_ms(1000), // Should not update
                delay_ms: None,
            },
            // TC2: Equal timestamp event
            TestCase {
                name: "equal timestamp event",
                time_initial: plus_ms(1000),
                input_events: vec![market_event(plus_ms(1000))],
                expected_time_exchange_last: plus_ms(1000), // Should maintain current time
                delay_ms: None,
            },
            // TC3: Multiple events in order
            TestCase {
                name: "multiple events in order",
                time_initial: time_base,
                input_events: vec![
                    market_event(plus_ms(1000)),
                    market_event(plus_ms(2000)),
                    market_event(plus_ms(3000)),
                ],
                expected_time_exchange_last: plus_ms(3000),
                delay_ms: Some(10), // Small delay between events
            },
            // TC4: Multiple events out of order
            TestCase {
                name: "multiple events out of order",
                time_initial: time_base,
                input_events: vec![
                    market_event(plus_ms(3000)),
                    market_event(plus_ms(1000)),
                    market_event(plus_ms(2000)),
                ],
                expected_time_exchange_last: plus_ms(3000),
                delay_ms: Some(10),
            },
            // TC5: Event with no timestamp
            TestCase {
                name: "event with no timestamp",
                time_initial: plus_ms(1000),
                input_events: vec![EngineEvent::Market(MarketStreamEvent::Reconnecting(
                    ExchangeId::BinanceSpot,
                ))],
                expected_time_exchange_last: plus_ms(1000), // Should not update
                delay_ms: None,
            },
            // TC6: Mixed events with and without timestamps
            TestCase {
                name: "mixed events with and without timestamps",
                time_initial: time_base,
                input_events: vec![
                    market_event(plus_ms(1000)),
                    EngineEvent::Market(MarketStreamEvent::Reconnecting(ExchangeId::BinanceSpot)),
                    market_event(plus_ms(2000)),
                ],
                expected_time_exchange_last: plus_ms(2000),
                delay_ms: Some(10),
            },
        ];

        for (index, test) in cases.iter().enumerate() {
            // Setup clock with initial time
            let mut clock = HistoricalClock::new(test.time_initial);

            // Process all events
            for event in test.input_events.iter() {
                clock.process(event);

                // Add delay if specified
                if let Some(delay) = test.delay_ms {
                    spin_sleep::sleep(std::time::Duration::from_millis(delay));
                }
            }

            assert_eq!(
                clock.inner.read().time_exchange_last,
                test.expected_time_exchange_last,
                "TC{} ({}) failed - incorrect time_exchange_last",
                index,
                test.name
            );
        }
    }

    #[test]
    fn test_historical_clock_time_delta_calculation() {
        let time_base = DateTime::<Utc>::MIN_UTC;
        let clock = HistoricalClock::new(time_base);

        // Get initial time
        let time_1 = clock.time();

        // Sleep to simulate time passing
        spin_sleep::sleep(std::time::Duration::from_millis(100));

        // Get time after delay
        let time_2 = clock.time();

        // Verify time has increased
        assert!(
            time_2 > time_1,
            "Historical clock time should increase with wall clock"
        );

        // Verify increase is reasonable (eg/ close to our sleep duration)
        let delta_ms = time_2.signed_duration_since(time_1).num_milliseconds();

        assert!(
            delta_ms >= 95 && delta_ms <= 105,
            "Historical clock time delta outside expected range"
        );
    }
}
