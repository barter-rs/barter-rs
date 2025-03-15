use std::collections::HashMap;
use barter::statistic::summary::instrument::TearSheetGenerator;
use barter_execution::order::id::StrategyId;
use barter_integration::collection::FnvIndexSet;

struct MultiStrategy {
    strategy_a: StrategyA,
    strategy_b: StrategyB,
}

struct StrategyA;
struct StrategyB;

struct MultiStrategyState {
    tear_sheets: FnvIndexSet<InstrumentTearSheets>
}

struct InstrumentTearSheets {
    tear: TearSheetGenerator,
    tear_by_strategy: HashMap<StrategyId, TearSheetGenerator>
}

// Todo:
//  - Function for Orders by Strategy? Or perhaps Orders should natively be by Strategy
//  - InstrumentFilter::Strategy?
//  - Perhaps trait InstrumentData should contain methods for each AccountEvent variant...?

#[tokio::main]
async fn main() {

}