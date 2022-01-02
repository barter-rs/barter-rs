pub mod error;
pub mod trader;

use std::collections::HashMap;
use crate::Market;
use crate::engine::error::EngineError;
use crate::engine::trader::Trader;
use crate::data::handler::{Continuer, MarketGenerator};
use crate::strategy::SignalGenerator;
use crate::portfolio::repository::PositionHandler;
use crate::portfolio::{FillUpdater, MarketUpdater, OrderGenerator};
use crate::portfolio::position::Position;
use crate::execution::FillGenerator;
use crate::statistic::summary::trading::TradingSummary;
use crate::statistic::summary::{PositionSummariser, TablePrinter};
use crate::event::{Event, MessageTransmitter};
use std::fmt::Debug;
use std::sync::{Mutex, Arc};
use std::thread;
use tokio::sync::{mpsc, oneshot};
use tracing::{info, warn, error};
use uuid::Uuid;

// Todo:
//  - Impl consistent structured logging in Engine & Trader
//  - Ensure i'm happy with where event Event & Command live (eg/ Balance is in event.rs)
//  - Add Deserialize to Event.
//  - Search for wrong indented Wheres
//  - Do I want to roll out Market instead of Exchange & Symbol in all Events? (can't for Position due to serde)
//  - Search for todo!() since I found one in /statistic/summary/pnl.rs
//  - Change trader event_q capacity! What does it need to be?
//  - Fix unwraps() - search code eg/ engine::send_open_positions
//  - Ensure I havn't lost any improvements I had on the other branches!
//  - Add unit test cases for update_from_fill tests (4 of them) which use get & set stats
//  - Make as much stuff Copy as can be - start in Statistics!
//  - Add comments where we see '/// Todo:' or similar
//  - Print summary for each Market, rather than as a total



/// Communicates a String is a message associated with a [`Command`].
pub type Message = String;

#[derive(Debug)]
pub enum Command {
    SendOpenPositions(oneshot::Sender<Result<Vec<Position>, EngineError>>), // Engine
    SendSummary(oneshot::Sender<Result<TradingSummary, EngineError>>),      // Engine
    Terminate(Message),                                                     // All Traders
    ExitAllPositions,                                                       // All Traders
    ExitPosition(Market),                                                   // Single Trader
}

/// Lego components for constructing an [`Engine`] via the new() constructor method.
#[derive(Debug)]
pub struct EngineLego<EventTx, Statistic, Portfolio, Data, Strategy, Execution>
where
    EventTx: MessageTransmitter<Event> + Debug  + Send,
    Statistic: PositionSummariser + TablePrinter,
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
    /// Statistics component that can generate a trading summary based on closed positions.
    pub statistics: Statistic,
    /// Shared-access to a global Portfolio instance.
    pub portfolio: Arc<Mutex<Portfolio>>,
    /// Collection of [`Trader`] instances that can concurrently trade a market pair on it's own thread.
    pub traders: Vec<Trader<EventTx, Portfolio, Data, Strategy, Execution>>,
    /// Todo:
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
    EventTx: MessageTransmitter<Event> + Debug,
    Statistic: PositionSummariser + TablePrinter,
    Portfolio: MarketUpdater + OrderGenerator + FillUpdater + Debug + Send,
    Data: Continuer + MarketGenerator + Debug + Send,
    Strategy: SignalGenerator + Debug + Send,
    Execution: FillGenerator + Debug + Send,
{
    /// Unique identifier for an [`Engine`] in Uuid v4 format. Used as a unique identifier seed for
    /// the Portfolio, Trader & Positions associated with this [`Engine`].
    engine_id: Uuid,
    /// mpsc::Receiver for receiving [`Command`]s from a remote source.
    command_rx: mpsc::Receiver<Command>,
    /// Statistics component that can generate a trading summary based on closed positions.
    statistics: Statistic,
    /// Shared-access to a global Portfolio instance that implements [`MarketUpdater`],
    /// [`OrderGenerator`] & [`FillUpdater`].
    portfolio: Arc<Mutex<Portfolio>>,
    /// Collection of [`Trader`] instances that can concurrently trade a market pair on it's own thread.
    traders: Vec<Trader<EventTx, Portfolio, Data, Strategy, Execution>>,
    /// Todo:
    trader_command_txs: HashMap<Market, mpsc::Sender<Command>>,
}

impl<EventTx, Statistic, Portfolio, Data, Strategy, Execution>
Engine<EventTx, Statistic, Portfolio, Data, Strategy, Execution>
where
    EventTx: MessageTransmitter<Event> + Debug  + Send + 'static,
    Statistic: PositionSummariser + TablePrinter,
    Portfolio: PositionHandler + MarketUpdater + OrderGenerator + FillUpdater + Debug + Send + 'static,
    Data: Continuer + MarketGenerator + Debug + Send + 'static,
    Strategy: SignalGenerator + Debug + Send + 'static,
    Execution: FillGenerator + Debug + Send + 'static,
{
    /// Constructs a new trading [`Engine`] instance using the provided [`EngineLego`].
    pub fn new(lego: EngineLego<EventTx, Statistic, Portfolio, Data, Strategy, Execution>) -> Self {
        info!(engine_id = &*format!("{}", lego.engine_id), "constructed new Engine instance");
        Self {
            engine_id: lego.engine_id,
            command_rx: lego.command_rx,
            statistics: lego.statistics,
            portfolio: lego.portfolio,
            traders: lego.traders,
            trader_command_txs: lego.trader_command_txs
        }
    }

    /// Builder to construct [`Engine`] instances.
    pub fn builder() -> EngineBuilder<EventTx, Statistic, Portfolio, Data, Strategy, Execution> {
        EngineBuilder::new()
    }

    /// Run the trading [`Engine`]. Spawns a thread for each [`Trader`] instance in the [`Engine`] and run
    /// the [`Trader`] event-loop. Asynchronously awaits a remote shutdown [`Message`]
    /// via the [`Engine`]'s termination_rx. After remote shutdown has been initiated, the trading
    /// period's statistics are generated & printed with the provided Statistic component.
    pub async fn run(mut self) {
        // Run Traders on threads & send notification when they have stopped organically
        let mut notify_traders_stopped = self.run_traders_new().await;

        loop {
            // Action received commands from remote, or wait for all Traders to stop organically
            tokio::select! {
                _ = notify_traders_stopped.recv() => {
                    break;
                },

                command = self.command_rx.recv() => {
                    if let Some(command) = command {
                        match command {
                            Command::SendOpenPositions(positions_rx) => {
                                self.send_open_positions(positions_rx);
                            },
                            Command::SendSummary(summary_rx) => {
                                self.send_summary(summary_rx);
                            },
                            Command::Terminate(message) => {
                                self.terminate_traders(message);
                                break;
                            },
                            Command::ExitPosition(market) => {
                                self.exit_position(market);
                            },
                            Command::ExitAllPositions => {
                                self.exit_all_positions();
                            },
                        }
                    } else {
                        // Terminate traders due to dropped receiver
                        break;
                    }
                }
            }
        };

        // Unlock Portfolio Mutex to access backtest information
        let mut portfolio = match self.portfolio.lock() {
            Ok(portfolio) => portfolio,
            Err(err) => {
                warn!("Mutex poisoned with error: {}", err);
                err.into_inner()
            }
        };

        // Generate TradingSummary
        match portfolio.get_exited_positions(&Uuid::new_v4()).unwrap() {
            None => info!("Backtest yielded no closed Positions - no TradingSummary available"),
            Some(closed_positions) => {
                self.statistics.generate_summary(&closed_positions);
                self.statistics.print();
            }
        }
    }

    /// Todo: Also deal w/ unwraps
    async fn run_traders_new(&mut self) -> mpsc::Receiver<bool> {
        // Extract Traders out of the Engine so we can move them into threads
        let traders = std::mem::replace(
            &mut self.traders, Vec::with_capacity(0)
        );

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
                handle.join().unwrap()
            }

            notify_tx.send(true).await.unwrap();
        });

        notify_rx
    }

    /// Todo:
    async fn send_open_positions(&self, positions_rx: oneshot::Sender<Result<Vec<Position>, EngineError>>) {
        let open_positions = self
            .portfolio
            .lock().unwrap()
            .get_open_positions(&self.engine_id, self.trader_command_txs.keys())
            .map_err(|err| EngineError::from(err));

        if positions_rx.send(open_positions).is_err() {
            warn!(why = "oneshot receiver dropped", "cannot action SendOpenPositions Command");
        }
    }

    /// Todo:
    fn send_summary(&self, summary_rx: oneshot::Sender<Result<TradingSummary, EngineError>>) {
        todo!()
    }
    /// Todo:
    fn terminate_traders(&self, message: Message) {
        todo!()
    }

    /// Todo:
    fn exit_position(&self, market: Market) {
        todo!()
    }

    /// Todo:
    fn exit_all_positions(&self) {
        todo!()
    }
}

/// Builder to construct [`Engine`] instances.
#[derive(Debug)]
pub struct EngineBuilder<EventTx, Statistic, Portfolio, Data, Strategy, Execution>
where
    EventTx: MessageTransmitter<Event> + Debug,
    Statistic: PositionSummariser + TablePrinter,
    Portfolio: MarketUpdater + OrderGenerator + FillUpdater + Debug + Send,
    Data: Continuer + MarketGenerator + Debug + Send,
    Strategy: SignalGenerator + Debug + Send,
    Execution: FillGenerator + Debug + Send,
{
    engine_id: Option<Uuid>,
    command_rx: Option<mpsc::Receiver<Command>>,
    statistics: Option<Statistic>,
    portfolio: Option<Arc<Mutex<Portfolio>>>,
    traders: Option<Vec<Trader<EventTx, Portfolio, Data, Strategy, Execution>>>,
    trader_command_txs: Option<HashMap<Market, mpsc::Sender<Command>>>,
}

impl<EventTx, Statistic, Portfolio, Data, Strategy, Execution>
EngineBuilder<EventTx, Statistic, Portfolio, Data, Strategy, Execution>
where
    EventTx: MessageTransmitter<Event> + Debug,
    Statistic: PositionSummariser + TablePrinter,
    Portfolio: MarketUpdater + OrderGenerator + FillUpdater + Debug + Send,
    Data: Continuer + MarketGenerator + Debug + Send,
    Strategy: SignalGenerator + Debug + Send,
    Execution: FillGenerator + Debug + Send,
{
    fn new() -> Self {
        Self {
            engine_id: None,
            command_rx: None,
            statistics: None,
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

    pub fn statistics(self, value: Statistic) -> Self {
        Self {
            statistics: Some(value),
            ..self
        }
    }

    pub fn portfolio(self, value: Arc<Mutex<Portfolio>>) -> Self {
        Self {
            portfolio: Some(value),
            ..self
        }
    }

    pub fn traders(self, value: Vec<Trader<EventTx, Portfolio, Data, Strategy, Execution>>) -> Self {
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


    pub fn build(self) -> Result<Engine<EventTx, Statistic, Portfolio, Data, Strategy, Execution>, EngineError> {
        let engine_id = self.engine_id.ok_or(EngineError::BuilderIncomplete)?;
        let command_rx = self.command_rx.ok_or(EngineError::BuilderIncomplete)?;
        let statistics = self.statistics.ok_or(EngineError::BuilderIncomplete)?;
        let portfolio = self.portfolio.ok_or(EngineError::BuilderIncomplete)?;
        let traders = self.traders.ok_or(EngineError::BuilderIncomplete)?;
        let trader_command_txs = self.trader_command_txs.ok_or(EngineError::BuilderIncomplete)?;

        Ok(Engine {
            engine_id,
            command_rx,
            statistics,
            portfolio,
            traders,
            trader_command_txs,
        })
    }
}