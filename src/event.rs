use crate::{
    execution::FillEvent,
    portfolio::{
        position::{Position, PositionExit, PositionUpdate},
        Balance, OrderEvent,
    },
    strategy::{Signal, SignalForceExit},
};
use barter_data::event::{DataKind, MarketEvent};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use tokio::sync::mpsc;
use tracing::warn;

/// Events that occur when bartering. [`MarketEvent`], [`Signal`], [`OrderEvent`], and
/// [`FillEvent`] are vital to the [`Trader`](crate::engine::trader::Trader) event loop, dictating
/// the trading sequence. The [`PositionExit`] Event is a representation of work done by the
/// system, and is useful for analysing performance & reconciliations.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Event {
    Market(MarketEvent<DataKind>),
    Signal(Signal),
    SignalForceExit(SignalForceExit),
    OrderNew(OrderEvent),
    OrderUpdate,
    Fill(FillEvent),
    PositionNew(Position),
    PositionUpdate(PositionUpdate),
    PositionExit(PositionExit),
    Balance(Balance),
}

/// Message transmitter for sending Barter messages to downstream consumers.
pub trait MessageTransmitter<Message> {
    /// Attempts to send a message to an external message subscriber.
    fn send(&mut self, message: Message);

    /// Attempts to send many messages to an external message subscriber.
    fn send_many(&mut self, messages: Vec<Message>);
}

/// Transmitter for sending Barter [`Event`]s to an external sink. Useful for event-sourcing,
/// real-time dashboards & general monitoring.
#[derive(Debug, Clone)]
pub struct EventTx {
    /// Flag to communicate if the external [`Event`] receiver has been dropped.
    receiver_dropped: bool,
    /// [`Event`] channel transmitter to send [`Event`]s to an external sink.
    event_tx: mpsc::UnboundedSender<Event>,
}

impl MessageTransmitter<Event> for EventTx {
    fn send(&mut self, message: Event) {
        if self.receiver_dropped {
            return;
        }

        if self.event_tx.send(message).is_err() {
            warn!(
                action = "setting receiver_dropped = true",
                why = "event receiver dropped",
                "cannot send Events"
            );
            self.receiver_dropped = true;
        }
    }

    fn send_many(&mut self, messages: Vec<Event>) {
        if self.receiver_dropped {
            return;
        }

        messages.into_iter().for_each(|message| {
            let _ = self.event_tx.send(message);
        })
    }
}

impl EventTx {
    /// Constructs a new [`EventTx`] instance using the provided channel transmitter.
    pub fn new(event_tx: mpsc::UnboundedSender<Event>) -> Self {
        Self {
            receiver_dropped: false,
            event_tx,
        }
    }
}
