use crate::portfolio::position::Position;
use crate::statistic::metric::{MetricRolling, ProfitLoss};
use std::collections::HashMap;
use std::fmt::Display;
use prettytable::{Row, Table};
use crate::statistic::error::StatisticError;

// Todo:
//  - to_tuple...?
//  - &'static associated constants?
//  - Do I want to bin Display impl in favour of another method in MetricRolling eg/ print() ?
//  - Do I even need to create the tear sheet with the symbol, just add it in when a Position arrives w/ new symbol

pub trait Summariser {
    const SUMMARY_ID: &'static str;
    fn update_summary(&mut self, position: &Position);
    fn print_table(&self);
}

pub type SymbolID = String;
pub type MetricID = String;

pub struct ProfitLossSheet {
    sheet: HashMap<SymbolID, ProfitLoss>,
}

impl Summariser for ProfitLossSheet {
    const SUMMARY_ID: &'static str = "Profit & Loss Sheet";

    fn update_summary(&mut self, position: &Position) {
        match self.sheet.get_mut(&*position.symbol) {
            None => {
                let mut first_symbol_pnl = ProfitLoss::init();
                first_symbol_pnl.update(position);
                self.sheet.insert(position.symbol.clone(), first_symbol_pnl);
            }
            Some(symbol_pnl) => {
                symbol_pnl.update(position);
            }
        }
    }

    fn print_table(&self) {
        let mut pnl_sheet = Table::new();

        pnl_sheet.set_titles(row!["",
            "Long Contracts", "Long PnL", "Long PnL Per Contract",
            "Short Contracts", "Short PnL", "Short PnL Per Contract",
            "Total Contracts", "Total PnL", "Total PnL Per Contract"
        ]);

        for (symbol, pnl_summary) in self.iter() {
            pnl_sheet.add_row(row![symbol,
                self.long_contracts, self.long_pnl, self.long_pnl_per_contract,
                self.short_contracts, self.short_pnl, self.short_pnl_per_contract,
                self.total_contracts, self.total_pnl, self.total_pnl_per_contract
            ]);
        }

        pnl_sheet.printstd();
    }
}

impl ProfitLossSheet {
    pub fn new() -> Self {
        Self {
            sheet: HashMap::<SymbolID, ProfitLoss>::new()
        }
    }
}

pub struct TearSheet<T> where T: MetricRolling {
    sheet: HashMap<SymbolID, HashMap<MetricID, T>>,
}

impl<T> Summariser for TearSheet<T> where T: MetricRolling + Display {
    const SUMMARY_ID: &'static str = "Tear Sheet";

    fn update_summary(&mut self, position: &Position) {
        if let Some(symbol_metrics) = self.sheet.get_mut(&*position.symbol) {
            symbol_metrics
                .values_mut()
                .map(|metric| T::update(metric, position))
                .next();
        }
        else {
            panic!("Encountered position.symbol that is not in the tear_sheet.symbols list")
        }
    }

    fn print_table(&self) {
        let mut tear_sheet = Table::new();
        let mut titles = vec![""];

        for (symbol, metrics) in self.sheet.iter() {
            for (metric, value) in metrics.iter() {
                titles.push(metric);
                tear_sheet.add_row(row![symbol, value]);

            }
        }

        tear_sheet.set_titles(Row::from(titles));
        tear_sheet.printstd();
    }
}

impl<T> TearSheet<T> where T: MetricRolling + Display + Clone {
    pub fn new() -> Self {
        Self {
            sheet: HashMap::<SymbolID, HashMap<MetricID, T>>::new()
        }
    }

    pub fn builder() -> TearSheetBuilder<T> {
        TearSheetBuilder::new()
    }

    pub fn generate_statistics(&mut self, positions: &Vec<Position>) {
        for position in positions {
            self.update_summary(position);
        }
    }
}

pub struct TearSheetBuilder<T> where T: MetricRolling {
    symbols: Option<Vec<String>>,
    metrics: HashMap<MetricID, T>,
}

impl<T> TearSheetBuilder<T> where T: MetricRolling + Clone {
    pub fn new() -> Self {
        Self {
            symbols: None,
            metrics: HashMap::<String, T>::new(),
        }
    }

    pub fn symbols(mut self, symbols: Vec<String>) -> Self {
        self.symbols = Some(symbols);
        self
    }

    pub fn metric(mut self, metric: T) -> Self {
        // Return without modification if metric name already exists in metrics Map
        if self.metrics.contains_key(&*T::METRIC_ID) {
            return self
        }

        // Insert new metric initial value into Map
        self.metrics.insert(String::from(T::METRIC_ID), metric);
        self
    }

    pub fn build(self) -> Result<TearSheet<T>, StatisticError> {
        if let Some(symbols) = self.symbols {

            // Validate at least one metric has been added
            if self.metrics.is_empty() {
                return Err(StatisticError::BuilderNoMetricsProvided)
            }

            let mut sheet = HashMap::with_capacity(symbols.len());

            for symbol in symbols.into_iter() {
                sheet.insert(symbol, self.metrics.clone());
            }

            Ok(TearSheet {
                sheet
            })

        } else {
            Err(StatisticError::BuilderIncomplete)
        }
    }
}

