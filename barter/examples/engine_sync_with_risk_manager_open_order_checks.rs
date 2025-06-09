use barter::{
    EngineEvent,
    engine::{
        clock::LiveClock,
        state::{
            EngineState,
            global::DefaultGlobalData,
            instrument::{
                data::{DefaultInstrumentMarketData, InstrumentDataState},
                filter::InstrumentFilter,
            },
            trading::TradingState,
        },
    },
    logging::init_logging,
    risk::{
        DefaultRiskManager, RiskApproved, RiskManager, RiskRefused,
        check::{
            CheckHigherThan, RiskCheck,
            util::{calculate_abs_percent_difference, calculate_quote_notional},
        },
    },
    statistic::time::Daily,
    strategy::DefaultStrategy,
    system::{
        builder::{AuditMode, EngineFeedMode, SystemArgs, SystemBuilder},
        config::SystemConfig,
    },
};
use barter_data::{
    streams::builder::dynamic::indexed::init_indexed_multi_exchange_market_stream,
    subscription::SubKind,
};
use barter_execution::order::{
    OrderKind,
    request::{OrderRequestCancel, OrderRequestOpen},
};
use barter_instrument::{index::IndexedInstruments, instrument::kind::InstrumentKind};
use derive_more::Constructor;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, fs::File, io::BufReader, marker::PhantomData, time::Duration};
use tracing::warn;

const FILE_PATH_SYSTEM_CONFIG: &str = "barter/examples/config/system_config.json";
const RISK_FREE_RETURN: Decimal = dec!(0.05);

const MAX_MARKET_ORDER_PRICE_PERCENT_FROM_MARKET: CheckHigherThan<Decimal> = CheckHigherThan {
    limit: dec!(0.1), // 10%
};

// All configured Instruments are quoted in usdt
const MAX_USDT_NOTIONAL_PER_ORDER: CheckHigherThan<Decimal> = CheckHigherThan {
    limit: dec!(50.0), // 50 usdt
};

/// Custom risk manager that implements risk checks for orders
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Deserialize, Serialize, Constructor)]
pub struct CustomRiskManager<State> {
    pub max_notional_per_order: CheckHigherThan<Decimal>,
    pub max_market_order_price_percent_from_market: CheckHigherThan<Decimal>,
    phantom: PhantomData<State>,
}

impl<State> Default for CustomRiskManager<State> {
    fn default() -> Self {
        Self {
            max_notional_per_order: MAX_USDT_NOTIONAL_PER_ORDER,
            max_market_order_price_percent_from_market: MAX_MARKET_ORDER_PRICE_PERCENT_FROM_MARKET,
            phantom: PhantomData::default(),
        }
    }
}

impl RiskManager
    for CustomRiskManager<EngineState<DefaultGlobalData, DefaultInstrumentMarketData>>
{
    type State = EngineState<DefaultGlobalData, DefaultInstrumentMarketData>;

    fn check(
        &self,
        state: &Self::State,
        cancels: impl IntoIterator<Item = OrderRequestCancel>,
        opens: impl IntoIterator<Item = OrderRequestOpen>,
    ) -> (
        impl IntoIterator<Item = RiskApproved<OrderRequestCancel>>,
        impl IntoIterator<Item = RiskApproved<OrderRequestOpen>>,
        impl IntoIterator<Item = RiskRefused<OrderRequestCancel>>,
        impl IntoIterator<Item = RiskRefused<OrderRequestOpen>>,
    ) {
        // Always approve cancel requests (no risk check for cancels)
        let approved_cancels = cancels
            .into_iter()
            .map(RiskApproved::new)
            .collect::<Vec<_>>();

        // Process open order requests with risk checks
        let (approved_opens, refused_opens): (Vec<_>, Vec<_>) = opens
            .into_iter()
            .fold((Vec::new(), Vec::new()), |(mut approved, mut refused), request_open| {
                // Find InstrumentState associated with OrderRequestOpen
                let instrument_state = state
                    .instruments
                    .instrument_index(&request_open.key.instrument);

                if let InstrumentKind::Option(_) = instrument_state.instrument.kind {
                    refused.push(RiskRefused::new(
                        request_open,
                        "RiskManager cannot check Options orders without a strike price"
                    ));
                    return (approved, refused);
                }

                // Calculate notional value in instrument quote currency
                let notional = calculate_quote_notional(
                    request_open.state.quantity,
                    request_open.state.price,
                    instrument_state.instrument.kind.contract_size(),
                ).expect("notional calculation overflowed");

                // Filter orders with a notional higher than current limits
                if let Err(error) = self.max_notional_per_order.check(&notional) {
                    warn!(
                        instrument = %instrument_state.instrument.name_internal,
                        ?request_open,
                        ?error,
                        "RiskManager filtered order: max_notional_per_instrument failed"
                    );
                    refused.push(RiskRefused::new(
                        request_open,
                        "RiskManager max_notional_per_instrument failed"
                    ));
                    return (approved, refused);
                }

                // Only need to make additional checks if OrderKind::Market, so can approve otherwise
                if OrderKind::Market != request_open.state.kind {
                    approved.push(RiskApproved::new(request_open));
                    return (approved, refused);
                }

                // Check there is an instrument market data price available
                let Some(market_price) = instrument_state.data.price() else {
                    warn!(
                        instrument = %instrument_state.instrument.name_internal,
                        ?request_open,
                        market_data = ?instrument_state.data,
                        "RiskManager filtered order: max_market_order_price_percent_from_market failed: no available instrument market price"
                    );
                    refused.push(RiskRefused::new(
                        request_open,
                        "RiskManager max_market_order_price_percent_from_market failed"
                    ));
                    return (approved, refused);
                };

                // Calculate percentage difference from the latest market price
                let price_diff_pct = calculate_abs_percent_difference(
                    request_open.state.price,
                    market_price,
                ).expect("price abs percent difference calculation overflowed");

                // Filter orders with price_diff_pct deviation from the latest market data price
                if let Err(error) = self.max_market_order_price_percent_from_market.check(&price_diff_pct) {
                    warn!(
                        instrument = %instrument_state.instrument.name_internal,
                        ?request_open,
                        ?error,
                        "RiskManager filtered order: max_market_order_price_percent_from_market failed"
                    );
                    refused.push(RiskRefused::new(
                        request_open,
                        "RiskManager max_market_order_price_percent_from_market failed",
                    ));
                    return (approved, refused);
                }

                // All checks passed, approve order
                approved.push(RiskApproved::new(request_open));
                (approved, refused)
            });

        (
            approved_cancels,
            approved_opens,
            std::iter::empty(),
            refused_opens,
        )
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialise Tracing
    init_logging();

    // Load SystemConfig
    let SystemConfig {
        instruments,
        executions,
    } = load_config()?;

    // Construct IndexedInstruments
    let instruments = IndexedInstruments::new(instruments);

    // Initialise MarketData Stream
    let market_stream = init_indexed_multi_exchange_market_stream(
        &instruments,
        &[SubKind::PublicTrades, SubKind::OrderBooksL1],
    )
    .await?;

    // Construct System Args
    let args = SystemArgs::new(
        &instruments,
        executions,
        LiveClock,
        DefaultStrategy::default(),
        DefaultRiskManager::default(),
        market_stream,
        DefaultGlobalData::default(),
        |_| DefaultInstrumentMarketData::default(),
    );

    // Build & run the full system:
    // See SystemBuilder for all configuration options
    let system = SystemBuilder::new(args)
        // Engine feed in Sync mode (Iterator input)
        .engine_feed_mode(EngineFeedMode::Iterator)
        // Audit feed is disabled (Engine does not send audits)
        .audit_mode(AuditMode::Disabled)
        // Engine starts with TradingState::Enabled
        .trading_state(TradingState::Enabled)
        // Build System, but don't start spawning tasks yet
        .build::<EngineEvent, _>()?
        // Init System, spawning component tasks on the current runtime
        .init_with_runtime(tokio::runtime::Handle::current())
        .await?;

    // Let the example run for 5 seconds...
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Before shutting down, CancelOrders and then ClosePositions
    system.cancel_orders(InstrumentFilter::None);
    system.close_positions(InstrumentFilter::None);

    // Shutdown
    let (engine, _shutdown_audit) = system.shutdown().await?;

    // Generate TradingSummary<Daily>
    let trading_summary = engine
        .trading_summary_generator(RISK_FREE_RETURN)
        .generate(Daily);

    // Print TradingSummary<Daily> to terminal (could save in a file, send somewhere, etc.)
    trading_summary.print_summary();

    Ok(())
}

fn load_config() -> Result<SystemConfig, Box<dyn std::error::Error>> {
    let file = File::open(FILE_PATH_SYSTEM_CONFIG)?;
    let reader = BufReader::new(file);
    let config = serde_json::from_reader(reader)?;
    Ok(config)
}
