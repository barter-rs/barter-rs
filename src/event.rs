use crate::data::market::MarketEvent;
use crate::strategy::signal::SignalEvent;
use crate::portfolio::order::OrderEvent;
use crate::portfolio::position::Position;
use crate::execution::fill::FillEvent;
use tokio::sync::mpsc;
use tracing::warn;

#[derive(Debug)]
pub enum Event {
    Market(MarketEvent),
    Signal(SignalEvent),
    Order(OrderEvent),
    Fill(FillEvent),
    ClosedPosition(Position)
}

#[derive(Debug)]
pub struct EventSink {
    event_tx: mpsc::UnboundedSender<Event>,
}

impl EventSink {
    pub fn new(event_tx: mpsc::UnboundedSender<Event>) -> Self {
        Self { event_tx }
    }

    pub fn send(&mut self, event: Event) {
        if self.event_tx.send(event).is_err() {
            warn!("EventSink receiver has been dropped & cannot send Events");
        }
    }
}
