use crate::data::market::MarketEvent;
use crate::execution::fill::FillEvent;
use crate::portfolio::order::OrderEvent;
use crate::portfolio::position::Position;
use crate::strategy::signal::SignalEvent;
use serde::Serialize;
use tokio::sync::mpsc;
use tracing::warn;

/// Events that occur when bartering. [MarketEvent], [SignalEvent], [OrderEvent], and [FillEvent]
/// are vital to the [Trader](crate::engine::trader::Trader) event loop, dictating the trading
/// sequence. The closed [Position] Event is a representation of work done by the system, and is
/// useful for analysing performance & reconciliations.
#[derive(Debug, Serialize)]
pub enum Event {
    Market(MarketEvent),
    Signal(SignalEvent),
    Order(OrderEvent),
    Fill(FillEvent),
    ClosedPosition(Position),
}

/// Sink for sending [Event]s to an external source. Useful for event-sourcing, real-time
/// dashboards & general monitoring.
#[derive(Debug, Clone)]
pub struct EventSink {
    event_tx: mpsc::UnboundedSender<Event>,
}

impl EventSink {
    /// Constructs a new [EventSink] instance using the provided channel transmitter.
    pub fn new(event_tx: mpsc::UnboundedSender<Event>) -> Self {
        Self { event_tx }
    }

    /// Attempts to send a message on the [EventSink]s channel transmitter.
    pub fn send(&mut self, event: Event) {
        if self.event_tx.send(event).is_err() {
            warn!("EventSink receiver has been dropped & cannot send Events");
        }
    }
}
