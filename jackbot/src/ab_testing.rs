use crate::backtest::{backtest, BacktestArgsConstant, BacktestArgsDynamic, BacktestSummary};
use crate::engine::state::EngineState;
use crate::error::JackbotError;
use std::sync::Arc;

pub async fn ab_test<MD, SI, SA, SB, Risk, GD, ID>(
    args: Arc<BacktestArgsConstant<MD, SI, EngineState<GD, ID>>>,
    strategy_a: BacktestArgsDynamic<SA, Risk>,
    strategy_b: BacktestArgsDynamic<SB, Risk>,
) -> Result<(BacktestSummary<SI>, BacktestSummary<SI>), JackbotError>
where
    MD: crate::backtest::market_data::BacktestMarketData<Kind = ID::MarketEventKind>,
    SI: crate::statistic::time::TimeInterval,
    SA: crate::strategy::algo::AlgoStrategy<State = EngineState<GD, ID>>
        + crate::strategy::close_positions::ClosePositionsStrategy<State = EngineState<GD, ID>>
        + crate::strategy::on_trading_disabled::OnTradingDisabled<
            crate::engine::clock::HistoricalClock,
            EngineState<GD, ID>,
            crate::execution::execution_tx::MultiExchangeTxMap,
            Risk,
        >
        + crate::strategy::on_disconnect::OnDisconnectStrategy<
            crate::engine::clock::HistoricalClock,
            EngineState<GD, ID>,
            crate::execution::execution_tx::MultiExchangeTxMap,
            Risk,
        >
        + Send
        + 'static,
    <SA as crate::strategy::on_trading_disabled::OnTradingDisabled<
        crate::engine::clock::HistoricalClock,
        EngineState<GD, ID>,
        crate::execution::execution_tx::MultiExchangeTxMap,
        Risk,
    >>::OnTradingDisabled: std::fmt::Debug + Clone + Send,
    <SA as crate::strategy::on_disconnect::OnDisconnectStrategy<
        crate::engine::clock::HistoricalClock,
        EngineState<GD, ID>,
        crate::execution::execution_tx::MultiExchangeTxMap,
        Risk,
    >>::OnDisconnect: std::fmt::Debug + Clone + Send,
    SB: crate::strategy::algo::AlgoStrategy<State = EngineState<GD, ID>>
        + crate::strategy::close_positions::ClosePositionsStrategy<State = EngineState<GD, ID>>
        + crate::strategy::on_trading_disabled::OnTradingDisabled<
            crate::engine::clock::HistoricalClock,
            EngineState<GD, ID>,
            crate::execution::execution_tx::MultiExchangeTxMap,
            Risk,
        >
        + crate::strategy::on_disconnect::OnDisconnectStrategy<
            crate::engine::clock::HistoricalClock,
            EngineState<GD, ID>,
            crate::execution::execution_tx::MultiExchangeTxMap,
            Risk,
        >
        + Send
        + 'static,
    <SB as crate::strategy::on_trading_disabled::OnTradingDisabled<
        crate::engine::clock::HistoricalClock,
        EngineState<GD, ID>,
        crate::execution::execution_tx::MultiExchangeTxMap,
        Risk,
    >>::OnTradingDisabled: std::fmt::Debug + Clone + Send,
    <SB as crate::strategy::on_disconnect::OnDisconnectStrategy<
        crate::engine::clock::HistoricalClock,
        EngineState<GD, ID>,
        crate::execution::execution_tx::MultiExchangeTxMap,
        Risk,
    >>::OnDisconnect: std::fmt::Debug + Clone + Send,
    Risk: crate::risk::RiskManager<State = EngineState<GD, ID>> + Send + 'static,
    GD: for<'a> crate::engine::Processor<&'a crate::data::event::MarketEvent<crate::instrument::instrument::InstrumentIndex, ID::MarketEventKind>>
        + for<'a> crate::engine::Processor<&'a crate::execution::AccountEvent>
        + std::fmt::Debug
        + Clone
        + Default
        + Send
        + 'static,
    ID: crate::engine::state::instrument::data::InstrumentDataState + Send + 'static,
{
    let res_a = backtest(Arc::clone(&args), strategy_a).await?;
    let res_b = backtest(args, strategy_b).await?;
    Ok((res_a, res_b))
}
