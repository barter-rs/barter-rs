use crate::data::handler::{Continuation, Continuer, MarketGenerator};
use crate::engine::error::EngineError;
use crate::engine::Command;
use crate::event::{Event, MessageTransmitter};
use crate::execution::FillGenerator;
use crate::portfolio::{FillUpdater, MarketUpdater, OrderGenerator};
use crate::strategy::signal::SignalForceExit;
use crate::strategy::SignalGenerator;
use crate::Market;
use serde::Serialize;
use std::collections::VecDeque;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::sync::Arc;
use parking_lot::Mutex;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TryRecvError;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Communicates a String represents a unique [`Trader`] identifier.
pub type TraderId = String;

/// Returns a unique identifier for a [`Trader`] given an engine_id, exchange & symbol.
pub fn determine_trader_id(engine_id: Uuid, exchange: &str, symbol: &str) -> TraderId {
    format!("{}_trader_{}_{}", engine_id, exchange, symbol)
}

/// Lego components for constructing a [`Trader`] via the new() constructor method.
#[derive(Debug)]
pub struct TraderLego<EventTx, Statistic, Portfolio, Data, Strategy, Execution>
where
    EventTx: MessageTransmitter<Event>,
    Statistic: Serialize,
    Portfolio: MarketUpdater + OrderGenerator + FillUpdater,
    Data: Continuer + MarketGenerator,
    Strategy: SignalGenerator,
    Execution: FillGenerator,
{
    /// Identifier for the [`Engine`] this [`Trader`] is associated with (1-to-many relationship)..
    pub engine_id: Uuid,
    /// Details the exchange (eg/ "binance") & pair symbol (eg/ btc_usd) this [`Trader`] is trading on.
    market: Market,
    /// mpsc::Receiver for receiving [`Command`]s from a remote source.
    pub command_rx: mpsc::Receiver<Command>,
    /// [`Event`] transmitter for sending every [`Event`] the [`Trader`] encounters to an external sink.
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
    _statistic_marker: PhantomData<Statistic>,
}

/// Trader instance capable of trading a single market pair with it's own Data Handler, Strategy &
/// Execution Handler, as well as shared access to a global Portfolio instance. A graceful remote
/// shutdown is made possible by sending a [`Command::Terminate`] to the Trader's
/// mpsc::Receiver command_rx.
#[derive(Debug)]
pub struct Trader<EventTx, Statistic, Portfolio, Data, Strategy, Execution>
where
    EventTx: MessageTransmitter<Event>,
    Statistic: Serialize + Send,
    Portfolio: MarketUpdater + OrderGenerator + FillUpdater,
    Data: Continuer + MarketGenerator + Send,
    Strategy: SignalGenerator + Send,
    Execution: FillGenerator + Send,
{
    /// Identifier for the [`Engine`] this [`Trader`] is associated with (1-to-many relationship).
    engine_id: Uuid,
    /// Details the exchange (eg/ "binance") & pair symbol (eg/ btc_usd) this [`Trader`] is trading on.
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
    _statistic_marker: PhantomData<Statistic>,
}

impl<EventTx, Statistic, Portfolio, Data, Strategy, Execution>
    Trader<EventTx, Statistic, Portfolio, Data, Strategy, Execution>
where
    EventTx: MessageTransmitter<Event>,
    Statistic: Serialize + Send,
    Portfolio: MarketUpdater + OrderGenerator + FillUpdater,
    Data: Continuer + MarketGenerator + Send,
    Strategy: SignalGenerator + Send,
    Execution: FillGenerator + Send,
{
    /// Constructs a new [`Trader`] instance using the provided [`TraderLego`].
    pub fn new(lego: TraderLego<EventTx, Statistic, Portfolio, Data, Strategy, Execution>) -> Self {
        info!(
            engine_id = &*lego.engine_id.to_string(),
            market = &*format!("{:?}", lego.market),
            "constructed new Trader instance"
        );

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
            _statistic_marker: PhantomData::default(),
        }
    }

    /// Builder to construct [`Trader`] instances.
    pub fn builder() -> TraderBuilder<EventTx, Statistic, Portfolio, Data, Strategy, Execution> {
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
                    Command::Terminate(_) => break 'trading,
                    Command::ExitPosition(market) => {
                        self.event_q
                            .push_back(Event::SignalForceExit(SignalForceExit::new(market)));
                    }
                    _ => continue,
                }
            }

            // If the trading loop should_continue, populate event_q with the next MarketEvent
            match self.data.can_continue() {
                Continuation::Continue => {
                    if let Some(market_event) = self.data.generate_market() {
                        self.event_tx.send(Event::Market(market_event.clone()));
                        self.event_q.push_back(Event::Market(market_event));
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

                        if let Some(position_update) = self
                            .portfolio
                            .lock()
                            .update_from_market(&market)
                            .expect("failed to update Portfolio from market")
                        {
                            self.event_tx.send(Event::PositionUpdate(position_update));
                        }
                    }

                    Event::Signal(signal) => {
                        if let Some(order) = self
                            .portfolio
                            .lock()
                            .generate_order(&signal)
                            .expect("failed to generate order")
                        {
                            self.event_tx.send(Event::OrderNew(order.clone()));
                            self.event_q.push_back(Event::OrderNew(order));
                        }
                    }

                    Event::SignalForceExit(signal_force_exit) => {
                        if let Some(order) = self
                            .portfolio
                            .lock()
                            .generate_exit_order(signal_force_exit)
                            .expect("failed to generate forced exit order")
                        {
                            self.event_tx.send(Event::OrderNew(order.clone()));
                            self.event_q.push_back(Event::OrderNew(order));
                        }
                    }

                    Event::OrderNew(order) => {
                        let fill = self
                            .execution
                            .generate_fill(&order)
                            .expect("failed to generate Fill");

                        self.event_tx.send(Event::Fill(fill.clone()));
                        self.event_q.push_back(Event::Fill(fill));
                    }

                    Event::Fill(fill) => {
                        let fill_side_effect_events = self
                            .portfolio
                            .lock()
                            .update_from_fill(&fill)
                            .expect("failed to update Portfolio from fill");

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
                debug!(
                    command = &*format!("{:?}", command),
                    "Trader received remote command"
                );
                Some(command)
            }
            Err(err) => match err {
                TryRecvError::Empty => None,
                TryRecvError::Disconnected => {
                    warn!(
                        action = "synthesising a Command::Terminate",
                        "remote Command transmitter has been dropped"
                    );
                    Some(Command::Terminate(
                        "remote command transmitter dropped".to_owned(),
                    ))
                }
            },
        }
    }
}

/// Builder to construct [`Trader`] instances.
#[derive(Debug, Default)]
pub struct TraderBuilder<EventTx, Statistic, Portfolio, Data, Strategy, Execution>
where
    EventTx: MessageTransmitter<Event>,
    Statistic: Serialize + Send,
    Portfolio: MarketUpdater + OrderGenerator + FillUpdater,
    Data: Continuer + MarketGenerator,
    Strategy: SignalGenerator,
    Execution: FillGenerator,
{
    engine_id: Option<Uuid>,
    market: Option<Market>,
    command_rx: Option<mpsc::Receiver<Command>>,
    event_sink: Option<EventTx>,
    portfolio: Option<Arc<Mutex<Portfolio>>>,
    data: Option<Data>,
    strategy: Option<Strategy>,
    execution: Option<Execution>,
    _statistic_marker: Option<PhantomData<Statistic>>,
}

impl<EventTx, Statistic, Portfolio, Data, Strategy, Execution>
    TraderBuilder<EventTx, Statistic, Portfolio, Data, Strategy, Execution>
where
    EventTx: MessageTransmitter<Event>,
    Statistic: Serialize + Send,
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
            portfolio: None,
            data: None,
            strategy: None,
            execution: None,
            _statistic_marker: None,
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

    pub fn build(
        self,
    ) -> Result<Trader<EventTx, Statistic, Portfolio, Data, Strategy, Execution>, EngineError> {
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
            event_q: VecDeque::with_capacity(2),
            portfolio,
            data,
            strategy,
            execution,
            _statistic_marker: PhantomData::default(),
        })
    }
}