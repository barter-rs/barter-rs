pub mod error;
pub mod trader;

use crate::data::handler::{Continuer, MarketGenerator};
use crate::engine::error::EngineError;
use crate::engine::trader::Trader;
use crate::event::{Event, MessageTransmitter};
use crate::execution::FillGenerator;
use crate::portfolio::position::Position;
use crate::portfolio::repository::{PositionHandler, StatisticHandler};
use crate::portfolio::{FillUpdater, MarketUpdater, OrderGenerator};
use crate::statistic::summary::TablePrinter;
use crate::strategy::SignalGenerator;
use crate::Market;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use std::thread;
use parking_lot::Mutex;
use tokio::sync::{mpsc, oneshot};
use tracing::{error, info, warn};
use uuid::Uuid;

// Enums:
// #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
// Structs w/0 f64:
// #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, Deserialize, Serialize)]
// Structs w/ f64:
// #[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Default, Deserialize, Serialize)]
// Structs that are objects:
// #[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]

// Todo:
//  0. Go through whole code and switch to combinators, and clean up anything obvious
//  1. Remove strange default() impls that I use for tests eg/ Position::default()
//  3. Ensure I am eagerly deriving as much as possible - especially enums! Work out the base derive:
//    '--> #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display, Default, Deserialize, Serialize)]
//  4.. Ensure everything is unit tested
//  5. Position::meta has duplicated exit timestamp in exit_timestamp & meta.equity_point.timestamp
//  6. Cleanup Config passing - seems like there is duplication eg/ Portfolio.starting_cash vs Portfolio.stats_config.starting_equity
//     '--> also can use references to markets to avoid cloning?
//  7. Clean up Results printing -> add titles if I print per. otherwise go back to old method
//  10. Update code examples & readme

// Todo: - Posting Testing:
//  2. Add more 'Balance' concept rather than start cash etc. BalanceHandler instead of Equity & Cash (fully copy)
//     '--> EquityPoint::from(Balance) which adds the timestamp, or does Balance hold the timestamp?
//  3. Ensure PositionNew, PositionUpdate & PositionExit is consistent -> create trait for Position?
//     '--> eg similar to Rust TAs Open, High etc
//  4. Go over Rust docs in key areas i've changed & traits etc

// Todo: 0.7.1
//  - Roll out consistent use of Market / Exchange / symbol (new types?)
//    '--> Remember (can't use Market instead of Exchange & Symbol for Position due to serde)
//    '--> eg/ portfolio.get_statistics(&self.market.market_id()) -> could market_id() return a ref?
//  - Make as much stuff Copy as can be - start in Statistics!
//  - If happy with it, impl Initialiser for all stats across the Statistics module.
//  - Add Deserialize to Event.
//  - Clean up OTT use of type aliases? New types instead?
//  - Add unit test cases for update_from_fill tests (4 of them) which use get & set stats
//  - Impl consistent structured logging in Engine & Trader
//   '--> Do I want to spans instead of multiple info logging? eg/ fetch_open_requests logs twice
//   '--> Where do I want to log things like Command::ExitPosition being actioned? In Engine or when we push SignalForceExit on to Q?

/// Communicates a String is a message associated with a [`Command`].
pub type Message = String;

/// Commands that can be actioned by an [`Engine`] and it's associated [`Trader`]s.
#[derive(Debug)]
pub enum Command {
    /// Fetches all the [`Engine`]'s open [`Position`]s and sends them on the provided
    /// `oneshot::Sender`.
    FetchOpenPositions(oneshot::Sender<Result<Vec<Position>, EngineError>>), // Engine

    /// Terminate every running [`Trader`] associated with this [`Engine`].
    Terminate(Message), // All Traders

    /// Exit every open [`Position`] associated with this [`Engine`].
    ExitAllPositions, // All Traders

    /// Exit a [`Position`]. Uses the [`Market`] provided to route this [`Command`] to the relevant
    /// [`Trader`] instance.
    ExitPosition(Market), // Single Trader
}

/// Lego components for constructing an [`Engine`] via the new() constructor method.
#[derive(Debug)]
pub struct EngineLego<EventTx, Statistic, Portfolio, Data, Strategy, Execution>
where
    EventTx: MessageTransmitter<Event> + Send,
    Statistic: Serialize + Send,
    Portfolio: MarketUpdater + OrderGenerator + FillUpdater + Send,
    Data: Continuer + MarketGenerator + Send,
    Strategy: SignalGenerator + Send,
    Execution: FillGenerator + Send,
{
    /// Unique identifier for an [`Engine`] in Uuid v4 format. Used as a unique identifier seed for
    /// the Portfolio, Trader & Positions associated with this [`Engine`].
    pub engine_id: Uuid,
    /// mpsc::Receiver for receiving [`Command`]s from a remote source.
    pub command_rx: mpsc::Receiver<Command>,
    /// Shared-access to a global Portfolio instance.
    pub portfolio: Arc<Mutex<Portfolio>>,
    /// Collection of [`Trader`] instances that can concurrently trade a market pair on it's own thread.
    pub traders: Vec<Trader<EventTx, Statistic, Portfolio, Data, Strategy, Execution>>,
    /// `HashMap` containing a [`Command`] transmitter for every [`Trader`] associated with this
    /// [`Engine`].
    pub trader_command_txs: HashMap<Market, mpsc::Sender<Command>>,
}

/// Multi-threaded Trading Engine capable of trading with an arbitrary number of [`Trader`] market
/// pairs. Each [`Trader`] operates on it's own thread and has it's own Data Handler, Strategy &
/// Execution Handler, as well as shared access to a global Portfolio instance. A graceful remote
/// shutdown is made possible by sending a [`Message`] to the Engine's broadcast::Receiver
/// termination_rx.
#[derive(Debug)]
pub struct Engine<EventTx, Statistic, Portfolio, Data, Strategy, Execution>
where
    EventTx: MessageTransmitter<Event>,
    Statistic: TablePrinter + Serialize + Send,
    Portfolio: PositionHandler
        + StatisticHandler<Statistic>
        + MarketUpdater
        + OrderGenerator
        + FillUpdater
        + Send
        + 'static,
    Data: Continuer + MarketGenerator + Send + 'static,
    Strategy: SignalGenerator + Send,
    Execution: FillGenerator + Send,
{
    /// Unique identifier for an [`Engine`] in Uuid v4 format. Used as a unique identifier seed for
    /// the Portfolio, Trader & Positions associated with this [`Engine`].
    engine_id: Uuid,
    /// mpsc::Receiver for receiving [`Command`]s from a remote source.
    command_rx: mpsc::Receiver<Command>,
    /// Shared-access to a global Portfolio instance that implements [`MarketUpdater`],
    /// [`OrderGenerator`] & [`FillUpdater`].
    portfolio: Arc<Mutex<Portfolio>>,
    /// Collection of [`Trader`] instances that can concurrently trade a market pair on it's own thread.
    traders: Vec<Trader<EventTx, Statistic, Portfolio, Data, Strategy, Execution>>,
    /// `HashMap` containing a [`Command`] transmitter for every [`Trader`] associated with this
    /// [`Engine`].
    trader_command_txs: HashMap<Market, mpsc::Sender<Command>>,
}

impl<EventTx, Statistic, Portfolio, Data, Strategy, Execution>
    Engine<EventTx, Statistic, Portfolio, Data, Strategy, Execution>
where
    EventTx: MessageTransmitter<Event> + Send + 'static,
    Statistic: TablePrinter + Serialize + Send + 'static,
    Portfolio: PositionHandler
        + StatisticHandler<Statistic>
        + MarketUpdater
        + OrderGenerator
        + FillUpdater
        + Send
        + 'static,
    Data: Continuer + MarketGenerator + Send,
    Strategy: SignalGenerator + Send + 'static,
    Execution: FillGenerator + Send + 'static,
{
    /// Constructs a new trading [`Engine`] instance using the provided [`EngineLego`].
    pub fn new(lego: EngineLego<EventTx, Statistic, Portfolio, Data, Strategy, Execution>) -> Self {
        info!(
            engine_id = &*format!("{}", lego.engine_id),
            "constructed new Engine instance"
        );
        Self {
            engine_id: lego.engine_id,
            command_rx: lego.command_rx,
            portfolio: lego.portfolio,
            traders: lego.traders,
            trader_command_txs: lego.trader_command_txs,
        }
    }

    /// Builder to construct [`Engine`] instances.
    pub fn builder() -> EngineBuilder<EventTx, Statistic, Portfolio, Data, Strategy, Execution> {
        EngineBuilder::new()
    }

    /// Run the trading [`Engine`]. Spawns a thread for each [`Trader`] to run on. Asynchronously
    /// receives [`Command`]s via the `command_rx` and actions them
    /// (eg/ terminate_traders, fetch_open_positions). If all of the [`Trader`]s stop organically
    /// (eg/ due to a finished [`MarketEvent`] feed), the [`Engine`] terminates & prints a summary
    /// for the trading session.
    pub async fn run(mut self) {
        // Run Traders on threads & send notification when they have stopped organically
        let mut notify_traders_stopped = self.run_traders().await;

        loop {
            // Action received commands from remote, or wait for all Traders to stop organically
            tokio::select! {
                _ = notify_traders_stopped.recv() => {
                    break;
                },

                command = self.command_rx.recv() => {
                    if let Some(command) = command {
                        match command {
                            Command::FetchOpenPositions(positions_tx) => {
                                self.fetch_open_positions(positions_tx).await;
                            },
                            Command::Terminate(message) => {
                                self.terminate_traders(message).await;
                                break;
                            },
                            Command::ExitPosition(market) => {
                                self.exit_position(market).await;
                            },
                            Command::ExitAllPositions => {
                                self.exit_all_positions().await;
                            },
                        }
                    } else {
                        // Terminate traders due to dropped receiver
                        break;
                    }
                }
            }
        }

        // Generate Statistics summary
        self.trader_command_txs.into_keys().for_each(|market| {
            self.portfolio
                .lock()
                .get_statistics(&market.market_id())
                .map(|statistic| statistic.print())
                .unwrap_or_else(|err| {
                    warn!(
                        error = &*format!("{:?}", err),
                        why = "failed to get statistics from Portfolio's repository",
                        "failed to generate Statistics summary for trading session"
                    )
                })
        });
    }

    /// Runs each [`Trader`] it's own thread. Sends a message on the returned `mpsc::Receiver<bool>`
    /// if all the [`Trader`]s have stopped organically (eg/ due to a finished [`MarketEvent`] feed).
    async fn run_traders(&mut self) -> mpsc::Receiver<bool> {
        // Extract Traders out of the Engine so we can move them into threads
        let traders = std::mem::take(&mut self.traders);

        // Run each Trader instance on it's own thread
        let mut thread_handles = Vec::with_capacity(traders.len());
        for trader in traders.into_iter() {
            let handle = thread::spawn(move || trader.run());
            thread_handles.push(handle);
        }

        // Create channel to notify the Engine when the Traders have stopped organically
        let (notify_tx, notify_rx) = mpsc::channel(1);

        // Create Task that notifies Engine when the Traders have stopped organically
        tokio::spawn(async move {
            for handle in thread_handles {
                if let Err(err) = handle.join() {
                    error!(
                        error = &*format!("{:?}", err),
                        "Trader thread has panicked during execution",
                    )
                }
            }

            let _ = notify_tx.send(true).await;
        });

        notify_rx
    }

    /// Fetches all the [`Engine`]'s open [`Position`]s and sends them on the provided
    /// `oneshot::Sender`.
    async fn fetch_open_positions(
        &self,
        positions_tx: oneshot::Sender<Result<Vec<Position>, EngineError>>,
    ) {
        let open_positions = self
            .portfolio
            .lock()
            .get_open_positions(&self.engine_id, self.trader_command_txs.keys())
            .map_err(EngineError::from);

        if positions_tx.send(open_positions).is_err() {
            warn!(
                why = "oneshot receiver dropped",
                "cannot action Command::FetchOpenPositions"
            );
        }
    }

    /// Terminate every running [`Trader`] associated with this [`Engine`].
    async fn terminate_traders(&self, message: Message) {
        // Firstly, exit all Positions
        self.exit_all_positions().await;
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        // Distribute Command::Terminate to all the Engine's Traders
        for (market, command_tx) in self.trader_command_txs.iter() {
            if command_tx
                .send(Command::Terminate(message.clone()))
                .await
                .is_err()
            {
                error!(
                    market = &*format!("{:?}", market),
                    why = "dropped receiver",
                    "failed to send Command::Terminate to Trader command_rx"
                );
            }
        }
    }

    /// Exit every open [`Position`] associated with this [`Engine`].
    async fn exit_all_positions(&self) {
        for (market, command_tx) in self.trader_command_txs.iter() {
            if command_tx
                .send(Command::ExitPosition(market.clone()))
                .await
                .is_err()
            {
                error!(
                    market = &*format!("{:?}", market),
                    why = "dropped receiver",
                    "failed to send Command::Terminate to Trader command_rx"
                );
            }
        }
    }

    /// Exit a [`Position`]. Uses the [`Market`] provided to route this [`Command`] to the relevant
    /// [`Trader`] instance.
    async fn exit_position(&self, market: Market) {
        if let Some((market_ref, command_tx)) = self.trader_command_txs.get_key_value(&market) {
            if command_tx
                .send(Command::ExitPosition(market))
                .await
                .is_err()
            {
                error!(
                    market = &*format!("{:?}", market_ref),
                    why = "dropped receiver",
                    "failed to send Command::Terminate to Trader command_rx"
                );
            }
        } else {
            warn!(
                market = &*format!("{:?}", market),
                why = "Engine has no trader_command_tx associated with provided Market",
                "failed to exit Position"
            );
        }
    }
}

/// Builder to construct [`Engine`] instances.
#[derive(Debug, Default)]
pub struct EngineBuilder<EventTx, Statistic, Portfolio, Data, Strategy, Execution>
where
    EventTx: MessageTransmitter<Event>,
    Statistic: Serialize + Send,
    Portfolio: MarketUpdater + OrderGenerator + FillUpdater + Send,
    Data: Continuer + MarketGenerator + Send,
    Strategy: SignalGenerator + Send,
    Execution: FillGenerator + Send,
{
    engine_id: Option<Uuid>,
    command_rx: Option<mpsc::Receiver<Command>>,
    portfolio: Option<Arc<Mutex<Portfolio>>>,
    traders: Option<Vec<Trader<EventTx, Statistic, Portfolio, Data, Strategy, Execution>>>,
    trader_command_txs: Option<HashMap<Market, mpsc::Sender<Command>>>,
}

impl<EventTx, Statistic, Portfolio, Data, Strategy, Execution>
    EngineBuilder<EventTx, Statistic, Portfolio, Data, Strategy, Execution>
where
    EventTx: MessageTransmitter<Event>,
    Statistic: TablePrinter + Serialize + Send,
    Portfolio: PositionHandler
        + StatisticHandler<Statistic>
        + MarketUpdater
        + OrderGenerator
        + FillUpdater
        + Send,
    Data: Continuer + MarketGenerator + Send,
    Strategy: SignalGenerator + Send,
    Execution: FillGenerator + Send,
{
    fn new() -> Self {
        Self {
            engine_id: None,
            command_rx: None,
            portfolio: None,
            traders: None,
            trader_command_txs: None,
        }
    }

    pub fn engine_id(self, value: Uuid) -> Self {
        Self {
            engine_id: Some(value),
            ..self
        }
    }

    pub fn command_rx(self, value: mpsc::Receiver<Command>) -> Self {
        Self {
            command_rx: Some(value),
            ..self
        }
    }

    pub fn portfolio(self, value: Arc<Mutex<Portfolio>>) -> Self {
        Self {
            portfolio: Some(value),
            ..self
        }
    }

    pub fn traders(
        self,
        value: Vec<Trader<EventTx, Statistic, Portfolio, Data, Strategy, Execution>>,
    ) -> Self {
        Self {
            traders: Some(value),
            ..self
        }
    }

    pub fn trader_command_txs(self, value: HashMap<Market, mpsc::Sender<Command>>) -> Self {
        Self {
            trader_command_txs: Some(value),
            ..self
        }
    }

    pub fn build(
        self,
    ) -> Result<Engine<EventTx, Statistic, Portfolio, Data, Strategy, Execution>, EngineError> {
        let engine_id = self.engine_id.ok_or(EngineError::BuilderIncomplete)?;
        let command_rx = self.command_rx.ok_or(EngineError::BuilderIncomplete)?;
        let portfolio = self.portfolio.ok_or(EngineError::BuilderIncomplete)?;
        let traders = self.traders.ok_or(EngineError::BuilderIncomplete)?;
        let trader_command_txs = self
            .trader_command_txs
            .ok_or(EngineError::BuilderIncomplete)?;

        Ok(Engine {
            engine_id,
            command_rx,
            portfolio,
            traders,
            trader_command_txs,
        })
    }
}