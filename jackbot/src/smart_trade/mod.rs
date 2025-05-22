use rust_decimal::Decimal;

pub mod trailing_take_profit;
pub mod profit_target;
pub mod trailing_stop;
pub mod multi_level_stop;
pub mod multi_level_take_profit;


pub use trailing_take_profit::TrailingTakeProfit;
pub use profit_target::ProfitTarget;
pub use trailing_stop::TrailingStop;
pub use multi_level_stop::MultiLevelStop;
pub use multi_level_take_profit::MultiLevelTakeProfit;

#[derive(Debug, Clone, PartialEq)]
pub enum SmartTradeSignal {
    TakeProfit(Decimal),
    StopLoss(Decimal),
    StopLevel(usize, Decimal),
}
