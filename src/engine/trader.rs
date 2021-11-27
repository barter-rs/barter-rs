use crate::data::handler::{Continuation, Continuer, MarketGenerator};
use crate::engine::error::EngineError;
use crate::engine::TerminationMessage;
use crate::event::{Event, EventSink};
use crate::execution::FillGenerator;
use crate::portfolio::{FillUpdater, MarketUpdater, OrderGenerator};
use crate::strategy::SignalGenerator;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::TryRecvError;
use tracing::{debug, info, warn};

/// Communicates if a process has received a termination command.
#[derive(Debug, Deserialize, Serialize)]
enum Termination {
    Received,
    Waiting,
}

/// Lego components for constructing a [Trader] via the new() constructor method.
#[derive(Debug)]
pub struct TraderLego<Portfolio, Data, Strategy, Execution>
where
    Portfolio: MarketUpdater + OrderGenerator + FillUpdater + Debug,
    Data: Continuer + MarketGenerator + Debug,
    Strategy: SignalGenerator + Debug,
    Execution: FillGenerator + Debug,
{
    /// broadcast::Receiver for receiving remote shutdown [TerminationMessage]s.
    pub termination_rx: broadcast::Receiver<TerminationMessage>,
    /// Sink for sending every [Event] the [Trader] encounters to an external source of choice.
    pub event_sink: EventSink,
    /// Shared-access to a global Portfolio instance that implements [MarketUpdater],
    /// [OrderGenerator] & [FillUpdater]. Generates [Event::Order]s, as well as reacts to
    /// [Event::Market]s, [Event::Signal]s, [Event::Fill]s.
    pub portfolio: Arc<Mutex<Portfolio>>,
    /// Data Handler implementing [Continuer] & [MarketGenerator], generates [Event::Market]s.
    pub data: Data,
    /// Strategy implementing [SignalGenerator], generates [Event::Signal]s.
    pub strategy: Strategy,
    /// Execution Handler implementing [FillGenerator], generates [Event::Fill]s.
    pub execution: Execution,
}

/// Trader instance capable of trading a single market pair with it's own Data Handler, Strategy &
/// Execution Handler, as well as shared access to a global Portfolio instance. A graceful remote
/// shutdown is made possible by sending a [TerminationMessage] to the Trader's broadcast::Receiver
/// termination_rx.
#[derive(Debug)]
pub struct Trader<Portfolio, Data, Strategy, Execution>
where
    Portfolio: MarketUpdater + OrderGenerator + FillUpdater,
    Data: Continuer + MarketGenerator + Send,
    Strategy: SignalGenerator + Send,
    Execution: FillGenerator + Send,
{
    /// broadcast::Receiver for receiving remote shutdown [TerminationMessage]s.
    termination_rx: broadcast::Receiver<TerminationMessage>,
    /// Sink for sending every [Event] the [Trader] encounters to an external source of choice.
    event_sink: EventSink,
    /// Queue for storing [Event]s used by the trading loop in the run() method.
    event_q: VecDeque<Event>,
    /// Shared-access to a global Portfolio instance that implements [MarketUpdater],
    /// [OrderGenerator] & [FillUpdater]. Generates [Event::Order]s, as well as reacts to
    /// [Event::Market]s, [Event::Signal]s, [Event::Fill]s.
    portfolio: Arc<Mutex<Portfolio>>,
    /// Data Handler implementing [Continuer] & [MarketGenerator], generates [Event::Market]s.
    data: Data,
    /// Strategy implementing [SignalGenerator], generates [Event::Signal]s.
    strategy: Strategy,
    /// Execution Handler implementing [FillGenerator], generates [Event::Fill]s.
    execution: Execution,
}

impl<Portfolio, Data, Strategy, Execution> Trader<Portfolio, Data, Strategy, Execution>
where
    Portfolio: MarketUpdater + OrderGenerator + FillUpdater + Debug,
    Data: Continuer + MarketGenerator + Debug + Send,
    Strategy: SignalGenerator + Debug + Send,
    Execution: FillGenerator + Debug + Send,
{
    /// Constructs a new [Trader] instance using the provided [TraderLego].
    pub fn new(lego: TraderLego<Portfolio, Data, Strategy, Execution>) -> Self {
        debug!(
            "Constructing a new Trader instance with TraderLego: {:?}",
            lego
        );
        Self {
            termination_rx: lego.termination_rx,
            event_sink: lego.event_sink,
            event_q: VecDeque::with_capacity(4),
            portfolio: lego.portfolio,
            data: lego.data,
            strategy: lego.strategy,
            execution: lego.execution,
        }
    }

    /// Builder to construct [Trader] instances.
    pub fn builder() -> TraderBuilder<Portfolio, Data, Strategy, Execution> {
        TraderBuilder::new()
    }

    /// Run trading event-loop for this [Trader] instance. Loop will run until [Trader] received a
    /// [TerminationMessage] via it's termination_rx broadcast::Receiver.
    pub fn run(mut self) {
        // Run trading loop for this Trader instance
        loop {
            // If the trading loop should_continue, populate event_q with the next MarketEvent
            match self.should_continue() {
                Continuation::Continue => {
                    if let Some(market_event) = self.data.generate_market() {
                        self.event_q.push_back(Event::Market(market_event))
                    }
                }
                Continuation::Stop => break,
            }

            // Handle Events (Market, Signal, Order, Fill) in the event_q
            // '--> While loop will break when event_q is empty and requires another MarketEvent
            while let Some(event) = self.event_q.pop_back() {
                match event {
                    Event::Market(market) => {
                        if let Some(signal) = self.strategy.generate_signal(&market) {
                            self.event_q.push_back(Event::Signal(signal));
                        }
                        self.portfolio
                            .lock()
                            .expect("Failed to unlock Mutex<Portfolio - poisoned")
                            .update_from_market(&market)
                            .expect("Failed to update portfolio from market");

                        // Send MarketEvent to EventSink
                        self.event_sink.send(Event::Market(market));
                    }

                    Event::Signal(signal) => {
                        if let Some(order) = self
                            .portfolio
                            .lock()
                            .expect("Failed to unlock Mutex<Portfolio - poisoned")
                            .generate_order(&signal)
                            .expect("Failed to generate order")
                        {
                            self.event_q.push_back(Event::Order(order));
                        }

                        // Send SignalEvent to EventSink
                        self.event_sink.send(Event::Signal(signal));
                    }

                    Event::Order(order) => {
                        self.event_q.push_back(Event::Fill(
                            self.execution
                                .generate_fill(&order)
                                .expect("Failed to generate fill"),
                        ));

                        // Send OrderEvent to EventSink
                        self.event_sink.send(Event::Order(order));
                    }

                    Event::Fill(fill) => {
                        // If FillEvent was an EXIT, send closed Position to EventSink
                        if let Some(closed_position) = self
                            .portfolio
                            .lock()
                            .expect("Failed to unlock Mutex<Portfolio - poisoned")
                            .update_from_fill(&fill)
                            .expect("Failed to update portfolio from fill")
                        {
                            self.event_sink.send(Event::ClosedPosition(closed_position));
                        }

                        // Send FillEvent to EventSink
                        self.event_sink.send(Event::Fill(fill));
                    }

                    _ => {}
                }
            }
        }
    }

    /// Determines whether the [Trader] instance's trading event-loop should continue. Returns a
    /// [Continuation] variant based on if the Data Handler can continue, as well as if a remote
    /// [TerminationMessage] has been received.
    fn should_continue(&mut self) -> Continuation {
        match (
            self.received_termination_command(),
            self.data.can_continue(),
        ) {
            (Termination::Waiting, Continuation::Continue) => Continuation::Continue,
            _ => Continuation::Stop,
        }
    }

    /// Returns a [Termination] variant depending on whether a remote [TerminationMessage] has
    /// been received.
    fn received_termination_command(&mut self) -> Termination {
        // Check termination channel to determine if Trader should continue
        match self.termination_rx.try_recv() {
            Ok(message) => {
                debug!(
                    "Stopping Trader after receiving termination message: {}",
                    message
                );
                Termination::Received
            }
            Err(err) => match err {
                TryRecvError::Empty => Termination::Waiting,
                TryRecvError::Closed => {
                    warn!(
                        "Stopping Trader after External termination transmitter dropped \
                                without sending a termination message"
                    );
                    Termination::Received
                }
                TryRecvError::Lagged(_) => {
                    info!("Stopping Trader - termination command received but message lost");
                    Termination::Received
                }
            },
        }
    }
}

/// Builder to construct [Trader] instances.
#[derive(Debug)]
pub struct TraderBuilder<Portfolio, Data, Strategy, Execution>
where
    Portfolio: MarketUpdater + OrderGenerator + FillUpdater,
    Data: Continuer + MarketGenerator,
    Strategy: SignalGenerator,
    Execution: FillGenerator,
{
    termination_rx: Option<broadcast::Receiver<TerminationMessage>>,
    event_sink: Option<EventSink>,
    event_q: Option<VecDeque<Event>>,
    portfolio: Option<Arc<Mutex<Portfolio>>>,
    data: Option<Data>,
    strategy: Option<Strategy>,
    execution: Option<Execution>,
}

impl<Portfolio, Data, Strategy, Execution> TraderBuilder<Portfolio, Data, Strategy, Execution>
where
    Portfolio: MarketUpdater + OrderGenerator + FillUpdater,
    Data: Continuer + MarketGenerator + Send,
    Strategy: SignalGenerator + Send,
    Execution: FillGenerator + Send,
{
    fn new() -> Self {
        Self {
            termination_rx: None,
            event_sink: None,
            event_q: None,
            portfolio: None,
            data: None,
            strategy: None,
            execution: None,
        }
    }

    pub fn termination_rx(self, value: broadcast::Receiver<TerminationMessage>) -> Self {
        Self {
            termination_rx: Some(value),
            ..self
        }
    }

    pub fn event_sink(self, value: EventSink) -> Self {
        Self {
            event_sink: Some(value),
            ..self
        }
    }

    pub fn portfolio(self, value: Arc<Mutex<Portfolio>>) -> Self {
        Self {
            portfolio: Some(value),
            ..self
        }
    }

    pub fn data(self, value: Data) -> Self {
        Self {
            data: Some(value),
            ..self
        }
    }

    pub fn strategy(self, value: Strategy) -> Self {
        Self {
            strategy: Some(value),
            ..self
        }
    }

    pub fn execution(self, value: Execution) -> Self {
        Self {
            execution: Some(value),
            ..self
        }
    }

    pub fn build(self) -> Result<Trader<Portfolio, Data, Strategy, Execution>, EngineError> {
        let termination_rx = self.termination_rx.ok_or(EngineError::BuilderIncomplete)?;
        let event_sink = self.event_sink.ok_or(EngineError::BuilderIncomplete)?;
        let portfolio = self.portfolio.ok_or(EngineError::BuilderIncomplete)?;
        let data = self.data.ok_or(EngineError::BuilderIncomplete)?;
        let strategy = self.strategy.ok_or(EngineError::BuilderIncomplete)?;
        let execution = self.execution.ok_or(EngineError::BuilderIncomplete)?;

        Ok(Trader {
            termination_rx,
            event_sink,
            event_q: VecDeque::with_capacity(4),
            portfolio,
            data,
            strategy,
            execution,
        })
    }
}
