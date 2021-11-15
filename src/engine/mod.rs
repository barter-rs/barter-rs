pub mod error;
pub mod trader;

use crate::data::handler::{Continuer, MarketGenerator};
use crate::engine::error::EngineError;
use crate::engine::trader::Trader;
use crate::execution::FillGenerator;
use crate::portfolio::repository::PositionHandler;
use crate::portfolio::{FillUpdater, MarketUpdater, OrderGenerator};
use crate::statistic::summary::{PositionSummariser, TablePrinter};
use crate::strategy::SignalGenerator;
use log::{info, warn};
use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::sync::{broadcast, oneshot};
use uuid::Uuid;

/// Communicative type alias to represent a termination message received via a termination channel.
pub type TerminationMessage = String;

/// Lego components for constructing an [Engine] via the new() constructor method.
#[derive(Debug)]
pub struct EngineLego<Statistic, Portfolio, Data, Strategy, Execution>
where
    Statistic: PositionSummariser + TablePrinter,
    Portfolio: MarketUpdater + OrderGenerator + FillUpdater + Send,
    Data: Continuer + MarketGenerator + Send,
    Strategy: SignalGenerator + Send,
    Execution: FillGenerator + Send,
{
    /// oneshot::Receiver for receiving remote shutdown [TerminationMessage]s.
    pub termination_rx: oneshot::Receiver<TerminationMessage>,
    /// broadcast::Sender for propagating remote shutdown [TerminationMessage]s to every [Trader] instance.
    pub traders_termination_tx: broadcast::Sender<TerminationMessage>,
    /// Statistics component that can generate a trading summary based on closed positions.
    pub statistics: Statistic,
    /// Shared-access to a global Portfolio instance.
    pub portfolio: Arc<Mutex<Portfolio>>,
    /// Collection of [Trader] instances that can concurrently trade a market pair on it's own thread.
    pub traders: Vec<Trader<Portfolio, Data, Strategy, Execution>>,
}

/// Multi-threaded Trading Engine capable of trading with an arbitrary number of [Trader] market
/// pairs. Each [Trader] operates on it's own thread and has it's own Data Handler, Strategy &
/// Execution Handler, as well as shared access to a global Portfolio instance. A graceful remote
/// shutdown is made possible by sending a [TerminationMessage] to the Engine's oneshot::Receiver
/// termination_rx.
#[derive(Debug)]
pub struct Engine<Statistic, Portfolio, Data, Strategy, Execution>
where
    Statistic: PositionSummariser + TablePrinter,
    Portfolio: MarketUpdater + OrderGenerator + FillUpdater + Debug + Send,
    Data: Continuer + MarketGenerator + Debug + Send,
    Strategy: SignalGenerator + Debug + Send,
    Execution: FillGenerator + Debug + Send,
{
    /// oneshot::Receiver for receiving remote shutdown [TerminationMessage]s.
    termination_rx: oneshot::Receiver<TerminationMessage>,
    /// broadcast::Sender for propagating remote shutdown [TerminationMessage]s to every [Trader] instance.
    traders_termination_tx: broadcast::Sender<TerminationMessage>,
    /// Statistics component that can generate a trading summary based on closed positions.
    statistics: Statistic,
    /// Shared-access to a global Portfolio instance that implements [MarketUpdater],
    /// [OrderGenerator] & [FillUpdater].
    portfolio: Arc<Mutex<Portfolio>>,
    /// Collection of [Trader] instances that can concurrently trade a market pair on it's own thread.
    traders: Vec<Trader<Portfolio, Data, Strategy, Execution>>,
}

impl<Statistic, Portfolio, Data, Strategy, Execution>
    Engine<Statistic, Portfolio, Data, Strategy, Execution>
where
    Statistic: PositionSummariser + TablePrinter,
    Portfolio:
        PositionHandler + MarketUpdater + OrderGenerator + FillUpdater + Debug + Send + 'static,
    Data: Continuer + MarketGenerator + Debug + Send + 'static,
    Strategy: SignalGenerator + Debug + Send + 'static,
    Execution: FillGenerator + Debug + Send + 'static,
{
    /// Constructs a new trading [Engine] instance using the provided [EngineLego].
    pub fn new(lego: EngineLego<Statistic, Portfolio, Data, Strategy, Execution>) -> Self {
        Self {
            termination_rx: lego.termination_rx,
            traders_termination_tx: lego.traders_termination_tx,
            statistics: lego.statistics,
            portfolio: lego.portfolio,
            traders: lego.traders,
        }
    }

    /// Builder to construct [Engine] instances.
    pub fn builder() -> EngineBuilder<Statistic, Portfolio, Data, Strategy, Execution> {
        EngineBuilder::new()
    }

    /// Run the trading [Engine]. Spawns a thread for each [Trader] instance in the [Engine] and run
    /// the [Trader] event-loop. Asynchronously awaits a remote shutdown [TerminationMessage]
    /// via the [Engine]'s termination_rx. After remote shutdown has been initiated, the trading
    /// period's statistics are generated & printed with the provided Statistic component.
    pub async fn run(mut self) {
        // Run each Trader instance on it's own thread
        self.traders.into_iter().for_each(|trader| {
            thread::spawn(move || trader.run());
        });

        // Await remote TerminationMessage command
        let termination_message = match self.termination_rx.await {
            Ok(message) => message,
            Err(_) => {
                let message =
                    String::from("Remote termination sender has been dropped - terminating Engine");
                warn!("{}", message);
                message
            }
        };

        // Propagate TerminationMessage command to every Trader instance
        if let Err(err) = self.traders_termination_tx.send(termination_message) {
            warn!(
                "Error occured while propagating TerminationMessage to Trader instances: {}",
                err
            );
        }

        // Unlock Portfolio Mutex to access backtest information
        let mut portfolio = match self.portfolio.lock() {
            Ok(portfolio) => portfolio,
            Err(err) => {
                warn!("Mutex poisoned with error: {}", err);
                err.into_inner()
            }
        };

        // Generate TradingSummary
        match portfolio.get_closed_positions(&Uuid::new_v4()).unwrap() {
            None => info!("Backtest yielded no closed Positions - no TradingSummary available"),
            Some(closed_positions) => {
                self.statistics.generate_summary(&closed_positions);
                self.statistics.print();
            }
        }
    }
}

/// Builder to construct [Engine] instances.
#[derive(Debug)]
pub struct EngineBuilder<Statistic, Portfolio, Data, Strategy, Execution>
where
    Statistic: PositionSummariser + TablePrinter,
    Portfolio: MarketUpdater + OrderGenerator + FillUpdater + Debug + Send,
    Data: Continuer + MarketGenerator + Debug + Send,
    Strategy: SignalGenerator + Debug + Send,
    Execution: FillGenerator + Debug + Send,
{
    termination_rx: Option<oneshot::Receiver<TerminationMessage>>,
    traders_termination_tx: Option<broadcast::Sender<TerminationMessage>>,
    statistics: Option<Statistic>,
    portfolio: Option<Arc<Mutex<Portfolio>>>,
    traders: Option<Vec<Trader<Portfolio, Data, Strategy, Execution>>>,
}

impl<Statistic, Portfolio, Data, Strategy, Execution>
    EngineBuilder<Statistic, Portfolio, Data, Strategy, Execution>
where
    Statistic: PositionSummariser + TablePrinter,
    Portfolio: MarketUpdater + OrderGenerator + FillUpdater + Debug + Send,
    Data: Continuer + MarketGenerator + Debug + Send,
    Strategy: SignalGenerator + Debug + Send,
    Execution: FillGenerator + Debug + Send,
{
    fn new() -> Self {
        Self {
            termination_rx: None,
            traders_termination_tx: None,
            statistics: None,
            portfolio: None,
            traders: None,
        }
    }

    pub fn termination_rx(self, value: oneshot::Receiver<TerminationMessage>) -> Self {
        Self {
            termination_rx: Some(value),
            ..self
        }
    }

    pub fn traders_termination_tx(self, value: broadcast::Sender<TerminationMessage>) -> Self {
        Self {
            traders_termination_tx: Some(value),
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

    pub fn traders(self, value: Vec<Trader<Portfolio, Data, Strategy, Execution>>) -> Self {
        Self {
            traders: Some(value),
            ..self
        }
    }

    pub fn build(
        self,
    ) -> Result<Engine<Statistic, Portfolio, Data, Strategy, Execution>, EngineError> {
        let termination_rx = self.termination_rx.ok_or(EngineError::BuilderIncomplete)?;
        let traders_termination_tx = self
            .traders_termination_tx
            .ok_or(EngineError::BuilderIncomplete)?;
        let statistics = self.statistics.ok_or(EngineError::BuilderIncomplete)?;
        let portfolio = self.portfolio.ok_or(EngineError::BuilderIncomplete)?;
        let traders = self.traders.ok_or(EngineError::BuilderIncomplete)?;

        Ok(Engine {
            termination_rx,
            traders_termination_tx,
            statistics,
            portfolio,
            traders,
        })
    }
}
