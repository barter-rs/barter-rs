use std::collections::HashMap;
use crate::statistic::profit_loss::ProfitLoss;
use crate::portfolio::position::Position;

pub struct Config {
    pub symbols: Vec<String>
}

pub struct HistoricStatistics {
    pub profit_loss: HashMap<String, Vec<ProfitLoss>>
}

impl HistoricStatistics {
    pub fn new(config: &Config, closed_positions: Vec<Position>) -> Self {
        // Allocate HashMap
        let mut profit_loss = HashMap::with_capacity(config.symbols.len());

        // Insert empty Vectors for each symbol
        for symbol in config.symbols.iter() {
            profit_loss.insert(symbol.clone(), Vec::new());
        }

        // Loop through closed positions
        for position in closed_positions.iter() {

            let symbol_pnl = profit_loss
                .get_mut(&*position.symbol)
                .unwrap();

            match symbol_pnl.is_empty() {
                true => {
                    symbol_pnl.push(ProfitLoss::init(position))
                }
                false => {
                    let prev_pnl = symbol_pnl
                        .get(symbol_pnl.len() - 1)
                        .unwrap();

                    symbol_pnl.push(prev_pnl.next(position))
                }
            }
        }

        Self {
            profit_loss
        }
    }

    pub fn display(self) {
        println!("PnL:\n");
        for (symbol, pnl) in self.profit_loss {

            let cum_pnl = pnl.get(pnl.len()-1).unwrap();

            println!("\n{}: {:?}\n", &symbol, cum_pnl.total_pnl)
        }
    }
}