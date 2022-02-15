// use barter::data::handler::historic::{HistoricCandleHandler, HistoricDataLego};
// use barter::engine::Engine;
// use barter::engine::trader::Trader;
// use barter::event::EventSink;
// use barter::execution::fill::Fees;
// use barter::execution::handler::{Config as ExecutionConfig, SimulatedExecution};
// use barter::portfolio::allocator::DefaultAllocator;
// use barter::portfolio::portfolio::MetaPortfolioOld;
// use barter::portfolio::risk::DefaultRisk;
// use barter::statistic::summary::trading::{Config as StatisticConfig, TradingSummary};
// use barter::strategy::strategy::{Config as StrategyConfig, RSIStrategy};
// use barter::portfolio::repository::in_memory::InMemoryRepository;
// use barter_data::model::Candle;
// use std::sync::{Arc, Mutex};
// use std::time::Duration;
// use tokio::sync::{broadcast, oneshot};
// use tokio::sync::mpsc::unbounded_channel;
// use uuid::Uuid;
//
// #[tokio::test]
// async fn engine_with_historic_data_stops_after_candles_finished() {
//     // Create termination channels that enable a graceful remote shutdown
//     let (_engine_termination_tx, engine_termination_rx) = oneshot::channel();
//     let (traders_termination_tx, _) = broadcast::channel(1);
//
//     // Create EventSink channel that enables listening to all Engine events in real-time
//     let (event_tx, _) = unbounded_channel();
//     let event_sink = EventSink::new(event_tx);
//
//     // Build global shared-state MetaPortfolio
//     let portfolio = Arc::new(Mutex::new(
//         MetaPortfolioOld::builder()
//             .id(Uuid::new_v4())
//             .starting_cash(10_000.0)
//             .repository(InMemoryRepository::new())
//             .allocation_manager(DefaultAllocator { default_order_value: 100.0 })
//             .risk_manager(DefaultRisk {})
//             .build_and_init()
//             .expect("failed to build & initialise MetaPortfolio"),
//     ));
//
//     // Build Trader(s)
//     let mut traders = Vec::new();
//     traders.push(
//         Trader::builder()
//             .termination_rx(traders_termination_tx.subscribe())
//             .event_sink(event_sink.clone())
//             .portfolio(Arc::clone(&portfolio))
//             .data(HistoricCandleHandler::new(HistoricDataLego {
//                 exchange: "binance",
//                 symbol: "btcusdt".to_string(),
//                 candles: vec![Candle::default()].into_iter()
//             }))
//             .strategy(RSIStrategy::new(StrategyConfig { rsi_period: 14 }))
//             .execution(SimulatedExecution::new(ExecutionConfig {
//                 simulated_fees_pct: Fees {
//                         exchange: 0.1,
//                         slippage: 0.05,
//                         network: 0.0,}
//                 }))
//             .build()
//             .expect("failed to build trader")
//     );
//
//     // Build Engine
//     let engine = Engine::builder()
//         .termination_rx(engine_termination_rx)
//         .traders_termination_tx(traders_termination_tx)
//         .statistics(TradingSummary::new(StatisticConfig {
//             starting_equity: 1000.0,
//             trading_days_per_year: 365,
//             risk_free_return: 0.0
//         }))
//         .portfolio(portfolio)
//         .traders(traders)
//         .build()
//         .expect("failed to build engine");
//
//     // Run Engine trading with timeout:
//     // If timeout before engine stops, remote termination_rx.await is incorrectly blocking the
//     // Engine from stopping even though the Traders have no more historical data to process
//     let timeout = Duration::from_millis(10);
//     let engine_run_future = engine.run();
//     let actual = tokio::time::timeout(timeout, engine_run_future).await;
//
//     assert!(
//         actual.is_ok(),
//         "Failed because engine_termination_rx.await is blocking the Engine from stopping"
//     )
// }
