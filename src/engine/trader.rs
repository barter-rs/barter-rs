use crate::data::handler::{Continuation, Continuer, MarketGenerator};
use crate::engine::error::EngineError;
use crate::event::{Event, MessageTransmitter};
use crate::execution::FillGenerator;
use crate::portfolio::{FillUpdater, MarketUpdater, OrderGenerator};
use crate::strategy::SignalGenerator;
use crate::engine::Command;
use crate::Market;
use crate::strategy::signal::SignalForceExit;
use std::collections::VecDeque;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TryRecvError;
use tracing::{debug, warn};
use uuid::Uuid;

/// Communicates a String represents a unique [`Trader`] identifier.
pub type TraderId = String;

/// Returns a unique identifier for a [`Trader`] given an engine_id, exchange & symbol.
pub fn determine_trader_id(engine_id: Uuid, exchange: &String, symbol: &String) -> TraderId {
    format!("{}_trader_{}_{}", engine_id, exchange, symbol)
}

/// Lego components for constructing a [`Trader`] via the new() constructor method.
#[derive(Debug)]
pub struct TraderLego<EventTx, Portfolio, Data, Strategy, Execution>
where
    EventTx: MessageTransmitter<Event> + Debug,
    Portfolio: MarketUpdater + OrderGenerator + FillUpdater + Debug,
    Data: Continuer + MarketGenerator + Debug,
    Strategy: SignalGenerator + Debug,
    Execution: FillGenerator + Debug,
{
    /// Couples this [`Trader`] instance to it's [`Engine`].
    pub engine_id: Uuid,
    /// Todo:
    market: Market,
    /// mpsc::Receiver for receiving [`Command`]s from a remote source.
    pub command_rx: mpsc::Receiver<Command>,
    /// [Event] transmitter for sending every [`Event`] the [`Trader`] encounters to an external sink.
    pub event_tx: EventTx,
    /// Shared-access to a global Portfolio instance that implements [`MarketUpdater`],
    /// [`OrderGenerator`] & [`FillUpdater`]. Generates [`Event::Order`]s, as well as reacts to
    /// [`Event::Market`]s, [`Event::Signal`]s, [`Event::Fill`]s.
    pub portfolio: Arc<Mutex<Portfolio>>,
    /// Data Handler implementing [`Continuer`] & [`MarketGenerator`], generates [`Event::Market`]s.
    pub data: Data,
    /// Strategy implementing [`SignalGenerator`], generates [`Event::Signal`]s.
    pub strategy: Strategy,
    /// Execution Handler implementing [`FillGenerator`], generates [`Event::Fill`]s.
    pub execution: Execution,
}

/// Trader instance capable of trading a single market pair with it's own Data Handler, Strategy &
/// Execution Handler, as well as shared access to a global Portfolio instance. A graceful remote
/// shutdown is made possible by sending a [`Command::Terminate`] to the Trader's
/// mpsc::Receiver command_rx.
#[derive(Debug)]
pub struct Trader<EventTx, Portfolio, Data, Strategy, Execution>
where
    EventTx: MessageTransmitter<Event> + Debug,
    Portfolio: MarketUpdater + OrderGenerator + FillUpdater,
    Data: Continuer + MarketGenerator + Send,
    Strategy: SignalGenerator + Send,
    Execution: FillGenerator + Send,
{
    /// Couples this [`Trader`] instance to it's [`Engine`].
    engine_id: Uuid,
    /// Todo:
    market: Market,
    /// mpsc::Receiver for receiving [`Command`]s from a remote source.
    command_rx: mpsc::Receiver<Command>,
    /// [`Event`] transmitter for sending every [`Event`] the [`Trader`] encounters to an external
    /// sink.
    event_tx: EventTx,
    /// Queue for storing [`Event`]s used by the trading loop in the run() method.
    event_q: VecDeque<Event>,
    /// Shared-access to a global Portfolio instance that implements [`MarketUpdater`],
    /// [`OrderGenerator`] & [`FillUpdater`]. Generates [`Event::Order`]s, as well as reacts to
    /// [`Event::Market`]s, [`Event::Signal`]s, [`Event::Fill`]s.
    portfolio: Arc<Mutex<Portfolio>>,
    /// Data Handler implementing [`Continuer`] & [`MarketGenerator`], generates [`Event::Market`]s.
    data: Data,
    /// Strategy implementing [`SignalGenerator`], generates [`Event::Signal`]s.
    strategy: Strategy,
    /// Execution Handler implementing [`FillGenerator`], generates [`Event::Fill`]s.
    execution: Execution,
}

impl<EventTx, Portfolio, Data, Strategy, Execution> Trader<EventTx, Portfolio, Data, Strategy, Execution>
where
    EventTx: MessageTransmitter<Event> + Debug,
    Portfolio: MarketUpdater + OrderGenerator + FillUpdater + Debug,
    Data: Continuer + MarketGenerator + Debug + Send,
    Strategy: SignalGenerator + Debug + Send,
    Execution: FillGenerator + Debug + Send,
{
    /// Constructs a new [`Trader`] instance using the provided [`TraderLego`].
    pub fn new(lego: TraderLego<EventTx, Portfolio, Data, Strategy, Execution>) -> Self {
        debug!(lego = &*format!("{:?}", lego), "constructed new Trader instance");

        Self {
            engine_id: lego.engine_id,
            market: lego.market,
            command_rx: lego.command_rx,
            event_tx: lego.event_tx,
            event_q: VecDeque::with_capacity(4),
            portfolio: lego.portfolio,
            data: lego.data,
            strategy: lego.strategy,
            execution: lego.execution,
        }
    }

    /// Builder to construct [`Trader`] instances.
    pub fn builder() -> TraderBuilder<EventTx, Portfolio, Data, Strategy, Execution> {
        TraderBuilder::new()
    }

    /// Run trading event-loop for this [`Trader`] instance. Loop will run until [`Trader`] receives
    /// a [`Command::Terminate`] via the mpsc::Receiver command_rx, or the data
    /// [`Continuer::can_continue`] returns [`Continuation::Stop`]
    pub fn run(mut self) {
        // Run trading loop for this Trader instance
        'trading: loop {

            // Check for new remote Commands before continuing to generate another MarketEvent
            while let Some(command) = self.receive_remote_command() {
                match command {
                    Command::Terminate(_) => {
                        break 'trading
                    },
                    Command::ExitPosition(market) => {
                        self.event_q.push_back(Event::SignalForceExit(
                            SignalForceExit::new(market)
                        ));
                    }
                    _ => continue,
                }
            }

            // If the trading loop should_continue, populate event_q with the next MarketEvent
            match self.data.can_continue() {
                Continuation::Continue => {
                    if let Some(market_event) = self.data.generate_market() {
                        self.event_q.push_back(Event::Market(market_event))
                    }
                }
                Continuation::Stop => break 'trading,
            }

            // Handle Events in the event_q
            // '--> While loop will break when event_q is empty and requires another MarketEvent
            while let Some(event) = self.event_q.pop_back() {

                match event {

                    Event::Market(market) => {
                        if let Some(signal) = self.strategy.generate_signal(&market) {
                            self.event_tx.send(Event::Signal(signal.clone()));
                            self.event_q.push_back(Event::Signal(signal));
                        }

                        if let Some(position_update) = self.portfolio
                            .lock()
                            .expect("Failed to unlock Mutex<Portfolio - poisoned")
                            .update_from_market(&market)
                            .expect("Failed to update portfolio from market") {
                            self.event_tx.send(Event::PositionUpdate(position_update));
                        }
                    }

                    Event::Signal(signal) => {
                        if let Some(order) = self
                            .portfolio
                            .lock()
                            .expect("Failed to unlock Mutex<Portfolio - poisoned")
                            .generate_order(&signal)
                            .expect("Failed to generate order")
                        {
                            self.event_tx.send(Event::OrderNew(order.clone()));
                            self.event_q.push_back(Event::OrderNew(order));
                        }
                    }

                    Event::SignalForceExit(signal_force_exit) => {
                        if let Some(order) = self
                            .portfolio
                            .lock()
                            .expect("Failed to unlock Mutex<Portfolio - poisoned")
                            .generate_exit_order(signal_force_exit)
                            .expect("Failed to generate forced exit order")
                        {
                            self.event_tx.send(Event::OrderNew(order.clone()));
                            self.event_q.push_back(Event::OrderNew(order));
                        }
                    }

                    Event::OrderNew(order) => {
                        let fill = self
                            .execution
                            .generate_fill(&order)
                            .expect("Failed to generate Fill");

                        self.event_tx.send(Event::Fill(fill.clone()));
                        self.event_q.push_back(Event::Fill(fill));
                    }

                    Event::Fill(fill) => {
                        let fill_side_effect_events = self.portfolio
                            .lock()
                            .expect("Failed to unlock Mutex<Portfolio - poisoned")
                            .update_from_fill(&fill)
                            .expect("Failed to update portfolio from fill");

                        self.event_tx.send_many(fill_side_effect_events);
                    }
                    _ => {}
                }
            }
        }
    }

    /// Returns a [`Command`] if one has been received.
    fn receive_remote_command(&mut self) -> Option<Command> {
        match self.command_rx.try_recv() {
            Ok(command) => {
                debug!(command = &*format!("{:?}", command), "Trader received remote command");
                Some(command)
            }
            Err(err) => match err {
                TryRecvError::Empty => None,
                TryRecvError::Disconnected => {
                    warn!(
                        action = "synthesising a Command::Terminate",
                        "remote Command transmitter has been dropped"
                    );
                    Some(Command::Terminate("remote command transmitter dropped".to_owned()))
                }
            },
        }
    }
}

/// Builder to construct [`Trader`] instances.
#[derive(Debug)]
pub struct TraderBuilder<EventTx, Portfolio, Data, Strategy, Execution>
where
    EventTx: MessageTransmitter<Event>,
    Portfolio: MarketUpdater + OrderGenerator + FillUpdater,
    Data: Continuer + MarketGenerator,
    Strategy: SignalGenerator,
    Execution: FillGenerator,
{
    engine_id: Option<Uuid>,
    market: Option<Market>,
    command_rx: Option<mpsc::Receiver<Command>>,
    event_sink: Option<EventTx>,
    event_q: Option<VecDeque<Event>>,
    portfolio: Option<Arc<Mutex<Portfolio>>>,
    data: Option<Data>,
    strategy: Option<Strategy>,
    execution: Option<Execution>,
}

impl<EventTx, Portfolio, Data, Strategy, Execution> TraderBuilder<EventTx, Portfolio, Data, Strategy, Execution>
where
    EventTx: MessageTransmitter<Event> + Debug,
    Portfolio: MarketUpdater + OrderGenerator + FillUpdater,
    Data: Continuer + MarketGenerator + Send,
    Strategy: SignalGenerator + Send,
    Execution: FillGenerator + Send,
{
    fn new() -> Self {
        Self {
            engine_id: None,
            market: None,
            command_rx: None,
            event_sink: None,
            event_q: None,
            portfolio: None,
            data: None,
            strategy: None,
            execution: None,
        }
    }

    pub fn engine_id(self, value: Uuid) -> Self {
        Self {
            engine_id: Some(value),
            ..self
        }
    }

    pub fn market(self, value: Market) -> Self {
        Self {
            market: Some(value),
            ..self
        }
    }

    pub fn command_rx(self, value: mpsc::Receiver<Command>) -> Self {
        Self {
            command_rx: Some(value),
            ..self
        }
    }

    pub fn event_sink(self, value: EventTx) -> Self {
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

    pub fn build(self) -> Result<Trader<EventTx, Portfolio, Data, Strategy, Execution>, EngineError> {
        let engine_id = self.engine_id.ok_or(EngineError::BuilderIncomplete)?;
        let market = self.market.ok_or(EngineError::BuilderIncomplete)?;
        let command_rx = self.command_rx.ok_or(EngineError::BuilderIncomplete)?;
        let event_tx = self.event_sink.ok_or(EngineError::BuilderIncomplete)?;
        let portfolio = self.portfolio.ok_or(EngineError::BuilderIncomplete)?;
        let data = self.data.ok_or(EngineError::BuilderIncomplete)?;
        let strategy = self.strategy.ok_or(EngineError::BuilderIncomplete)?;
        let execution = self.execution.ok_or(EngineError::BuilderIncomplete)?;

        Ok(Trader {
            engine_id,
            market,
            command_rx,
            event_tx,
            event_q: VecDeque::with_capacity(4),
            portfolio,
            data,
            strategy,
            execution,
        })
    }
}