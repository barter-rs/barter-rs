use super::{error::EngineError, Command};
use crate::{
    data::{Feed, MarketGenerator},
    event::{Event, MessageTransmitter},
    execution::ExecutionClient,
    portfolio::{FillUpdater, MarketUpdater, OrderGenerator},
    strategy::{SignalForceExit, SignalGenerator},
};
use barter_data::event::{DataKind, MarketEvent};
use barter_integration::model::Market;
use parking_lot::Mutex;
use serde::Serialize;
use std::{collections::VecDeque, fmt::Debug, marker::PhantomData, sync::Arc};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Lego components for constructing a [`Trader`] via the new() constructor method.
#[derive(Debug)]
pub struct TraderLego<EventTx, Statistic, Portfolio, Data, Strategy, Execution>
where
    EventTx: MessageTransmitter<Event>,
    Statistic: Serialize + Send,
    Portfolio: MarketUpdater + OrderGenerator + FillUpdater,
    Data: MarketGenerator<MarketEvent<DataKind>>,
    Strategy: SignalGenerator,
    Execution: ExecutionClient,
{
    /// Identifier for the [`Engine`](super::Engine) this [`Trader`] is associated with
    /// (1-to-many relationship).
    pub engine_id: Uuid,
    /// Communicates the unique [`Market`] this [`Trader`] is bartering on.
    pub market: Market,
    /// mpsc::Receiver for receiving [`Command`]s from a remote source.
    pub command_rx: mpsc::Receiver<Command>,
    /// [`Event`] transmitter for sending every [`Event`] the [`Trader`] encounters to an external sink.
    pub event_tx: EventTx,
    /// Shared-access to a global Portfolio instance that implements [`MarketUpdater`],
    /// [`OrderGenerator`] & [`FillUpdater`].
    pub portfolio: Arc<Mutex<Portfolio>>,
    /// Data handler that implements [`MarketGenerator`].
    pub data: Data,
    /// Strategy that implements [`SignalGenerator`].
    pub strategy: Strategy,
    /// Execution handler that implements [`ExecutionClient`].
    pub execution: Execution,
    _statistic_marker: PhantomData<Statistic>,
}

/// Trader instance capable of trading a single market pair with it's own Data Handler, Strategy &
/// Execution Handler, as well as shared access to a global Portfolio instance. It has a many-to-1
/// relationship with an Engine/Portfolio. A graceful remote shutdown is made possible by sending
/// a [`Command::Terminate`] to the Trader's
/// mpsc::Receiver command_rx.
#[derive(Debug)]
pub struct Trader<EventTx, Statistic, Portfolio, Data, Strategy, Execution>
where
    EventTx: MessageTransmitter<Event>,
    Statistic: Serialize + Send,
    Portfolio: MarketUpdater + OrderGenerator + FillUpdater,
    Data: MarketGenerator<MarketEvent<DataKind>> + Send,
    Strategy: SignalGenerator + Send,
    Execution: ExecutionClient + Send,
{
    /// Identifier for the [`Engine`](super::Engine) this [`Trader`] is associated with
    /// (1-to-many relationship).
    engine_id: Uuid,
    /// Communicates the unique [`Market`] this [`Trader`] is bartering on.
    market: Market,
    /// `mpsc::Receiver` for receiving [`Command`]s from a remote source.
    command_rx: mpsc::Receiver<Command>,
    /// [`Event`] transmitter for sending every [`Event`] the [`Trader`] encounters to an external
    /// sink.
    event_tx: EventTx,
    /// Queue for storing [`Event`]s used by the trading loop in the run() method.
    event_q: VecDeque<Event>,
    /// Shared-access to a global Portfolio instance that implements [`MarketUpdater`],
    /// [`OrderGenerator`] & [`FillUpdater`].
    portfolio: Arc<Mutex<Portfolio>>,
    /// Data handler that implements [`MarketGenerator`].
    data: Data,
    /// Strategy that implements [`SignalGenerator`].
    strategy: Strategy,
    /// Execution handler that implements [`ExecutionClient`].
    execution: Execution,
    _statistic_marker: PhantomData<Statistic>,
}

impl<EventTx, Statistic, Portfolio, Data, Strategy, Execution>
    Trader<EventTx, Statistic, Portfolio, Data, Strategy, Execution>
where
    EventTx: MessageTransmitter<Event>,
    Statistic: Serialize + Send,
    Portfolio: MarketUpdater + OrderGenerator + FillUpdater,
    Data: MarketGenerator<MarketEvent<DataKind>> + Send,
    Strategy: SignalGenerator + Send,
    Execution: ExecutionClient + Send,
{
    /// Constructs a new [`Trader`] instance using the provided [`TraderLego`].
    pub fn new(lego: TraderLego<EventTx, Statistic, Portfolio, Data, Strategy, Execution>) -> Self {
        info!(
            engine_id = %lego.engine_id,
            market = ?lego.market,
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

    /// Run the trading event-loop for this [`Trader`] instance. Loop will run until [`Trader`]
    /// receives a [`Command::Terminate`] via the mpsc::Receiver command_rx, or the
    /// [`MarketGenerator`] yields [`Feed::Finished`].
    pub fn run(mut self) {
        // Run trading loop for this Trader instance
        'trading: loop {
            // Check for new remote Commands before continuing to generate another MarketEvent
            while let Some(command) = self.receive_remote_command() {
                match command {
                    Command::Terminate(_) => break 'trading,
                    Command::ExitPosition(market) => {
                        self.event_q
                            .push_back(Event::SignalForceExit(SignalForceExit::from(market)));
                    }
                    _ => continue,
                }
            }

            // If the Feed<MarketEvent> yields, populate event_q with the next MarketEvent
            match self.data.next() {
                Feed::Next(market) => {
                    self.event_tx.send(Event::Market(market.clone()));
                    self.event_q.push_back(Event::Market(market));
                }
                Feed::Unhealthy => {
                    warn!(
                        engine_id = %self.engine_id,
                        market = ?self.market,
                        action = "continuing while waiting for healthy Feed",
                        "MarketFeed unhealthy"
                    );
                    continue 'trading;
                }
                Feed::Finished => break 'trading,
            }

            // Handle Events in the event_q
            // '--> While loop will break when event_q is empty and requires another MarketEvent
            while let Some(event) = self.event_q.pop_front() {
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

            debug!(
                engine_id = &*self.engine_id.to_string(),
                market = &*format!("{:?}", self.market),
                "Trader trading loop stopped"
            );
        }
    }

    /// Returns a [`Command`] if one has been received.
    fn receive_remote_command(&mut self) -> Option<Command> {
        match self.command_rx.try_recv() {
            Ok(command) => {
                debug!(
                    engine_id = &*self.engine_id.to_string(),
                    market = &*format!("{:?}", self.market),
                    command = &*format!("{:?}", command),
                    "Trader received remote command"
                );
                Some(command)
            }
            Err(err) => match err {
                mpsc::error::TryRecvError::Empty => None,
                mpsc::error::TryRecvError::Disconnected => {
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
    Data: MarketGenerator<MarketEvent<DataKind>>,
    Strategy: SignalGenerator,
    Execution: ExecutionClient,
{
    engine_id: Option<Uuid>,
    market: Option<Market>,
    command_rx: Option<mpsc::Receiver<Command>>,
    event_tx: Option<EventTx>,
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
    Data: MarketGenerator<MarketEvent<DataKind>> + Send,
    Strategy: SignalGenerator + Send,
    Execution: ExecutionClient + Send,
{
    fn new() -> Self {
        Self {
            engine_id: None,
            market: None,
            command_rx: None,
            event_tx: None,
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

    pub fn event_tx(self, value: EventTx) -> Self {
        Self {
            event_tx: Some(value),
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
        Ok(Trader {
            engine_id: self
                .engine_id
                .ok_or(EngineError::BuilderIncomplete("engine_id"))?,
            market: self
                .market
                .ok_or(EngineError::BuilderIncomplete("market"))?,
            command_rx: self
                .command_rx
                .ok_or(EngineError::BuilderIncomplete("command_rx"))?,
            event_tx: self
                .event_tx
                .ok_or(EngineError::BuilderIncomplete("event_tx"))?,
            event_q: VecDeque::with_capacity(2),
            portfolio: self
                .portfolio
                .ok_or(EngineError::BuilderIncomplete("portfolio"))?,
            data: self.data.ok_or(EngineError::BuilderIncomplete("data"))?,
            strategy: self
                .strategy
                .ok_or(EngineError::BuilderIncomplete("strategy"))?,
            execution: self
                .execution
                .ok_or(EngineError::BuilderIncomplete("execution"))?,
            _statistic_marker: PhantomData::default(),
        })
    }
}
